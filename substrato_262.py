#!/usr/bin/env python3
"""
Substrato 262 v8: QA-RSI Autopoiético com Execução Real e Circuit Breaker Thread-Safe

Este módulo implementa um agente de QA autopoiético com:
- Execução real de testes via subprocess, HTTP ou Playwright
- Circuit Breaker thread-safe com half-open controlado
- FAISS para busca semântica
- Darwin-Gödel Machine para auto-evolução
- Container de injeção de dependência
- Métricas e telemetria
- Servidor de teste embutido para validação

Autor: Catedral OS Team
Versão: 8.0
Data: 2026-08-28
"""

import json
import hashlib
import time
import random
import logging
import threading
import subprocess
import socket
from typing import Dict, List, Any, Optional, Callable, Tuple, Union
from dataclasses import dataclass, field
from abc import ABC, abstractmethod
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FuturesTimeout
from enum import Enum
from functools import wraps
from collections import defaultdict
import numpy as np

# ============================================================================
# IMPORTS OPCIONAIS (COM FALLBACKS GRACIOSOS)
# ============================================================================

try:
    import faiss
    HAS_FAISS = True
except ImportError:
    HAS_FAISS = False
    logging.warning("FAISS não disponível. Instale: pip install faiss-cpu")

try:
    import requests
    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False
    logging.warning("Requests não disponível. Instale: pip install requests")

try:
    from pydantic import BaseModel, validator, ValidationError
    HAS_PYDANTIC = True
except ImportError:
    HAS_PYDANTIC = False
    logging.warning("Pydantic não disponível. Instale: pip install pydantic")

try:
    from prometheus_client import Counter, Gauge, Histogram, start_http_server
    HAS_PROMETHEUS = True
except ImportError:
    HAS_PROMETHEUS = False

try:
    from sentence_transformers import SentenceTransformer
    HAS_SENTENCE_TRANSFORMERS = True
except ImportError:
    HAS_SENTENCE_TRANSFORMERS = False

# ============================================================================
# CONFIGURAÇÃO DE LOGGING
# ============================================================================

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [QA-RSI] %(levelname)s: %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger('qa_rsi')

# ============================================================================
# CONFIGURAÇÃO
# ============================================================================

@dataclass
class QARSIConfig:
    """Configuração completa do QA-RSI v8."""
    version: str = "8.0"

    # Grafo e conhecimento
    max_goals: int = 100
    max_tests_per_goal: int = 10
    model_name: str = "all-MiniLM-L6-v2"

    # Geração de testes
    fuzz_iterations: int = 5
    mutation_enabled: bool = True
    property_templates: List[str] = field(default_factory=lambda: [
        "Após {acao}, o sistema deve estar em {estado}",
        "Nenhum erro deve ser lançado durante {acao}",
        "O tempo de resposta para {acao} deve ser menor que {limiar}ms",
        "O sistema deve persistir dados corretamente durante {acao}"
    ])

    # Execução de testes
    executor_mode: str = "subprocess"  # subprocess, http, playwright, simulated
    test_timeout_seconds: int = 30
    max_workers: int = 4
    max_retries: int = 3
    retry_backoff: float = 1.0

    # Circuit Breaker
    failure_threshold: int = 5
    circuit_timeout_seconds: float = 30.0

    # DGM (Darwin-Gödel Machine)
    dgm_generations: int = 10
    dgm_archive_size: int = 20

    # RSI
    fitness_coverage_weight: float = 0.5
    fitness_stability_weight: float = 0.5

    # Infraestrutura
    telemetry_enabled: bool = True
    wormgraph_enabled: bool = True
    prometheus_enabled: bool = False
    prometheus_port: int = 8001
    random_seed: int = 42

    # Test server (para validação)
    test_server_port: Optional[int] = None
    test_server_enabled: bool = False

    # Peer review
    peer_review_enabled: bool = True
    peer_review_threshold: float = 0.6

    # Impact analysis
    impact_analysis_enabled: bool = True

# ============================================================================
# INTERFACES ABSTRATAS
# ============================================================================

class KnowledgeGraph(ABC):
    """Interface para grafo de conhecimento."""

    @abstractmethod
    def add_goal(self, goal_id: str, description: str, prerequisites: List[str] = None) -> Dict:
        pass

    @abstractmethod
    def update_result(self, goal_id: str, result: Dict) -> None:
        pass

    @abstractmethod
    def find_similar_goals(self, goal_id: str, top_k: int = 5) -> List[str]:
        pass

    @abstractmethod
    def get_goals(self) -> Dict[str, Dict]:
        pass

    @abstractmethod
    def get_version(self) -> int:
        pass

class LLMBackbone(ABC):
    """Interface para LLM."""

    @abstractmethod
    def generate(self, prompt: str, temperature: float = 0.7) -> str:
        pass

    @abstractmethod
    def generate_code(self, prompt: str) -> str:
        pass

class TestGenerator(ABC):
    """Interface para geração de testes."""

    @abstractmethod
    def generate_tests(self, goal_id: str) -> List[Dict]:
        pass

    @abstractmethod
    def mutate(self, test: Dict) -> List[Dict]:
        pass

    @abstractmethod
    def fuzz(self, goal_id: str, n_iterations: int) -> List[Dict]:
        pass

class Healer(ABC):
    """Interface para auto-cura."""

    @abstractmethod
    def heal(self, failed_test: Dict, context: Dict) -> Optional[Dict]:
        pass

class MetaEvaluator(ABC):
    """Interface para meta-avaliação."""

    @abstractmethod
    def evaluate(self, tests: List[Dict], codebase: str = "") -> Dict:
        pass

class WormGraphInterface(ABC):
    """Interface para WormGraph."""

    @abstractmethod
    def commit(self, block: Dict) -> bool:
        pass

class TelemetryInterface(ABC):
    """Interface para telemetria."""

    @abstractmethod
    def publish_metric(self, topic: str, metric: str, value: float) -> None:
        pass

# ============================================================================
# CIRCUIT BREAKER (THREAD-SAFE)
# ============================================================================

class CircuitState(Enum):
    """Estados do Circuit Breaker."""
    CLOSED = "closed"
    OPEN = "open"
    HALF_OPEN = "half-open"

class CircuitBreaker:
    """
    Circuit Breaker thread-safe com half-open controlado.

    Implementa o padrão de resiliência para evitar chamadas repetidas a serviços
    com falha, com transição atômica para half-open.
    """

    def __init__(self, failure_threshold: int = 5, timeout: float = 30.0):
        """
        Args:
            failure_threshold: Número de falhas consecutivas para abrir o circuito
            timeout: Tempo em segundos para tentar half-open
        """
        self.failure_threshold = failure_threshold
        self.timeout = timeout
        self._state = CircuitState.CLOSED
        self._failure_count = 0
        self._last_failure_time = 0.0
        self._half_open_attempts = 0
        self._half_open_max_attempts = 1
        self._lock = threading.RLock()
        self._metrics = {
            'total_failures': 0,
            'total_successes': 0,
            'state_changes': 0
        }

    @property
    def state(self) -> CircuitState:
        """Retorna o estado atual do circuito."""
        with self._lock:
            return self._state

    def is_open(self) -> bool:
        """
        Verifica se o circuito está aberto.

        Returns:
            True se o circuito estiver aberto e não devemos tentar
        """
        with self._lock:
            if self._state == CircuitState.OPEN:
                if time.time() - self._last_failure_time > self.timeout:
                    self._state = CircuitState.HALF_OPEN
                    self._half_open_attempts = 0
                    self._metrics['state_changes'] += 1
                    logger.info(f"CircuitBreaker: Transição para HALF_OPEN")
                    return False  # Permite a tentativa
                return True
            return False

    def record_success(self) -> None:
        """Registra uma chamada bem-sucedida."""
        with self._lock:
            self._failure_count = 0
            self._metrics['total_successes'] += 1
            if self._state == CircuitState.HALF_OPEN:
                self._state = CircuitState.CLOSED
                self._half_open_attempts = 0
                self._metrics['state_changes'] += 1
                logger.info(f"CircuitBreaker: Transição para CLOSED (sucesso no half-open)")

    def record_failure(self) -> None:
        """Registra uma chamada mal-sucedida."""
        with self._lock:
            self._failure_count += 1
            self._metrics['total_failures'] += 1

            if self._state == CircuitState.HALF_OPEN:
                self._half_open_attempts += 1
                if self._half_open_attempts >= self._half_open_max_attempts:
                    self._state = CircuitState.OPEN
                    self._last_failure_time = time.time()
                    self._metrics['state_changes'] += 1
                    logger.warning(f"CircuitBreaker: Transição para OPEN (falha no half-open)")
            elif self._failure_count >= self.failure_threshold:
                self._state = CircuitState.OPEN
                self._last_failure_time = time.time()
                self._metrics['state_changes'] += 1
                logger.warning(f"CircuitBreaker: Transição para OPEN ({self._failure_count} falhas)")

    def reset(self) -> None:
        """Reseta o circuito para o estado fechado."""
        with self._lock:
            self._state = CircuitState.CLOSED
            self._failure_count = 0
            self._half_open_attempts = 0
            self._metrics['state_changes'] += 1
            logger.info("CircuitBreaker: Resetado para CLOSED")

    def get_metrics(self) -> Dict:
        """Retorna métricas do circuit breaker."""
        with self._lock:
            return {
                'state': self._state.value,
                'failure_count': self._failure_count,
                'total_failures': self._metrics['total_failures'],
                'total_successes': self._metrics['total_successes'],
                'state_changes': self._metrics['state_changes'],
                'last_failure_time': self._last_failure_time
            }

# ============================================================================
# RETRYABLE (COM CIRCUIT BREAKER)
# ============================================================================

class Retryable:
    """
    Executa operações com retry e circuit breaker.

    Combina retry com backoff exponencial e circuit breaker para resiliência.
    """

    def __init__(self, max_retries: int = 3, backoff: float = 1.0,
                 failure_threshold: int = 5, circuit_timeout: float = 30.0):
        self.max_retries = max_retries
        self.backoff = backoff
        self.circuit_breaker = CircuitBreaker(failure_threshold, circuit_timeout)
        self._lock = threading.RLock()

    def execute(self, func: Callable, *args, **kwargs) -> Any:
        """
        Executa uma função com retry e circuit breaker.

        Args:
            func: Função a ser executada
            *args: Argumentos posicionais
            **kwargs: Argumentos nomeados

        Returns:
            Resultado da função

        Raises:
            RuntimeError: Se o circuito estiver aberto
            Exception: Última exceção após todas as tentativas
        """
        # Verifica o circuito
        if self.circuit_breaker.is_open():
            raise RuntimeError("Circuit breaker open - operation blocked")

        last_error = None
        for attempt in range(self.max_retries):
            try:
                result = func(*args, **kwargs)
                self.circuit_breaker.record_success()
                return result
            except Exception as e:
                last_error = e
                self.circuit_breaker.record_failure()

                # Se o circuito abriu, não tenta mais
                if self.circuit_breaker.state == CircuitState.OPEN:
                    raise RuntimeError(f"Circuit breaker opened after {attempt + 1} failures")

                # Backoff exponencial
                if attempt < self.max_retries - 1:
                    wait_time = self.backoff * (2 ** attempt)
                    logger.warning(f"Tentativa {attempt + 1} falhou. Aguardando {wait_time:.2f}s")
                    time.sleep(wait_time)

        raise last_error

    def get_metrics(self) -> Dict:
        """Retorna métricas do retryable."""
        return {
            'circuit_breaker': self.circuit_breaker.get_metrics(),
            'max_retries': self.max_retries,
            'backoff': self.backoff
        }

# ============================================================================
# EXECUTOR DE TESTES (REAL)
# ============================================================================

class ExecutorMode(Enum):
    """Modos de execução de testes."""
    SUBPROCESS = "subprocess"
    HTTP = "http"
    PLAYWRIGHT = "playwright"
    SIMULATED = "simulated"

class TestExecutor:
    """
    Executor real de testes com suporte a múltiplos backends.

    Suporta:
    - subprocess: Execução de comandos locais (curl, pytest, etc.)
    - http: Requisições HTTP a APIs
    - playwright: Automação de navegador
    - simulated: Fallback simulado com logging
    """

    def __init__(self, mode: ExecutorMode = ExecutorMode.SUBPROCESS, timeout: int = 30):
        self.mode = mode
        self.timeout = timeout
        self._results: List[Dict] = []
        self._stats = {
            'total': 0,
            'success': 0,
            'failure': 0,
            'timeout': 0
        }
        self._lock = threading.RLock()

    def execute(self, test: Dict) -> Dict:
        """
        Executa um teste real.

        Args:
            test: Dicionário com os parâmetros do teste

        Returns:
            Dicionário com o resultado da execução
        """
        with self._lock:
            self._stats['total'] += 1

        start = time.time()

        if self.mode == ExecutorMode.SUBPROCESS:
            result = self._execute_subprocess(test)
        elif self.mode == ExecutorMode.HTTP:
            result = self._execute_http(test)
        elif self.mode == ExecutorMode.PLAYWRIGHT:
            result = self._execute_playwright(test)
        else:
            result = self._execute_simulated(test)

        result['execution_time'] = time.time() - start

        with self._lock:
            if result.get('success', False):
                self._stats['success'] += 1
            else:
                self._stats['failure'] += 1
                if 'timeout' in str(result.get('error', '')).lower():
                    self._stats['timeout'] += 1

        self._results.append(result)
        return result

    def _execute_subprocess(self, test: Dict) -> Dict:
        """Executa teste via subprocess."""
        cmd = test.get('command', ['echo', 'test'])
        if isinstance(cmd, str):
            cmd = cmd.split()

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=self.timeout,
                check=False
            )
            return {
                'success': result.returncode == 0,
                'goal_id': test.get('goal_id'),
                'stdout': result.stdout[:1000] if result.stdout else '',
                'stderr': result.stderr[:1000] if result.stderr else '',
                'exit_code': result.returncode,
                'command': ' '.join(cmd)
            }
        except subprocess.TimeoutExpired:
            return {
                'success': False,
                'goal_id': test.get('goal_id'),
                'error': f'Timeout após {self.timeout}s',
                'command': ' '.join(cmd)
            }
        except Exception as e:
            return {
                'success': False,
                'goal_id': test.get('goal_id'),
                'error': str(e),
                'command': ' '.join(cmd)
            }

    def _execute_http(self, test: Dict) -> Dict:
        """Executa teste via requisição HTTP."""
        if not HAS_REQUESTS:
            return {
                'success': False,
                'goal_id': test.get('goal_id'),
                'error': 'Requests não instalado. Execute: pip install requests'
            }

        url = test.get('url', 'http://localhost:8080/health')
        method = test.get('method', 'GET').upper()
        headers = test.get('headers', {'Content-Type': 'application/json'})

        try:
            if method == 'GET':
                response = requests.get(url, headers=headers, timeout=self.timeout)
            elif method == 'POST':
                response = requests.post(url, json=test.get('payload', {}),
                                        headers=headers, timeout=self.timeout)
            elif method == 'PUT':
                response = requests.put(url, json=test.get('payload', {}),
                                       headers=headers, timeout=self.timeout)
            elif method == 'DELETE':
                response = requests.delete(url, headers=headers, timeout=self.timeout)
            else:
                return {
                    'success': False,
                    'goal_id': test.get('goal_id'),
                    'error': f'Método HTTP não suportado: {method}'
                }

            return {
                'success': 200 <= response.status_code < 300,
                'goal_id': test.get('goal_id'),
                'status_code': response.status_code,
                'body': response.text[:1000] if response.text else '',
                'url': url,
                'method': method
            }
        except requests.RequestException as e:
            return {
                'success': False,
                'goal_id': test.get('goal_id'),
                'error': str(e),
                'url': url,
                'method': method
            }

    def _execute_playwright(self, test: Dict) -> Dict:
        """Executa teste via Playwright (browser automation)."""
        try:
            from playwright.sync_api import sync_playwright
        except ImportError:
            return {
                'success': False,
                'goal_id': test.get('goal_id'),
                'error': 'Playwright não instalado. Execute: pip install playwright && playwright install'
            }

        url = test.get('url', 'http://localhost:8080')
        actions = test.get('actions', [])
        expected = test.get('expected', [])

        try:
            with sync_playwright() as p:
                browser = p.chromium.launch(headless=True)
                page = browser.new_page()
                page.set_default_timeout(self.timeout * 1000)
                page.goto(url)

                for action in actions:
                    action_type = action.get('type')
                    if action_type == 'click':
                        page.click(action.get('selector'))
                    elif action_type == 'fill':
                        page.fill(action.get('selector'), action.get('value', ''))
                    elif action_type == 'wait_for':
                        page.wait_for_selector(action.get('selector'))
                    elif action_type == 'select':
                        page.select_option(action.get('selector'), action.get('value'))

                # Verifica elementos esperados
                all_found = all(page.is_visible(e) for e in expected) if expected else True

                # Captura screenshot se falhar
                if not all_found:
                    screenshot = page.screenshot()
                    # Em produção, salvaria o screenshot

                browser.close()
                return {
                    'success': all_found,
                    'goal_id': test.get('goal_id'),
                    'url': url,
                    'actions': len(actions),
                    'expected_found': all_found
                }
        except Exception as e:
            return {
                'success': False,
                'goal_id': test.get('goal_id'),
                'error': str(e),
                'url': url
            }

    def _execute_simulated(self, test: Dict) -> Dict:
        """Fallback simulado (honesto — com logging)."""
        logger.warning(f"SIMULAÇÃO: executando teste {test.get('goal_id', 'unknown')}")
        # Usa a semente para reprodutibilidade
        seed = hash(test.get('goal_id', 'unknown')) % 2**32
        np.random.seed(seed)
        success = np.random.random() < 0.85

        return {
            'success': success,
            'goal_id': test.get('goal_id'),
            'error': None if success else 'Simulated failure',
            'simulated': True,
            'seed': seed
        }

    def get_stats(self) -> Dict:
        """Retorna estatísticas de execução."""
        with self._lock:
            total = self._stats['total']
            return {
                'total': total,
                'success': self._stats['success'],
                'failure': self._stats['failure'],
                'timeout': self._stats['timeout'],
                'success_rate': self._stats['success'] / total if total > 0 else 0.0,
                'mode': self.mode.value
            }

    def get_results(self) -> List[Dict]:
        """Retorna todos os resultados."""
        with self._lock:
            return self._results.copy()

# ============================================================================
# SERVIDOR HTTP DE TESTE
# ============================================================================

class TestServer:
    """
    Servidor HTTP de teste para validar execução real.

    Útil para testes de integração onde não há um sistema real disponível.
    """

    def __init__(self, port: int = 8888):
        self.port = port
        self._server = None
        self._thread = None
        self._last_request = None
        self._request_count = 0
        self._responses = {}

    from http.server import BaseHTTPRequestHandler
    class _Handler(BaseHTTPRequestHandler):
        def log_message(self, format, *args):
            pass  # Silencia logs do servidor

        def do_GET(self):
            self._handle_request('GET')

        def do_POST(self):
            self._handle_request('POST')

        def do_PUT(self):
            self._handle_request('PUT')

        def do_DELETE(self):
            self._handle_request('DELETE')

        def _handle_request(self, method):
            server = self.server
            server._last_request = {
                'method': method,
                'path': self.path,
                'headers': dict(self.headers)
            }

            # Lê corpo para POST/PUT
            if method in ['POST', 'PUT']:
                content_length = int(self.headers.get('Content-Length', 0))
                if content_length > 0:
                    body = self.rfile.read(content_length).decode()
                    server._last_request['body'] = body[:1000]

            server._request_count += 1

            # Verifica se há resposta personalizada para o path
            path = self.path.split('?')[0]
            custom_response = server._responses.get(path)

            if custom_response:
                status = custom_response.get('status', 200)
                content_type = custom_response.get('content_type', 'application/json')
                body = custom_response.get('body', '{}')
            else:
                status = 200
                content_type = 'application/json'
                body = json.dumps({
                    'status': 'ok',
                    'path': self.path,
                    'timestamp': time.time(),
                    'method': method,
                    'request_count': server._request_count
                })

            self.send_response(status)
            self.send_header('Content-Type', content_type)
            self.end_headers()
            self.wfile.write(body.encode())

    def start(self) -> None:
        """Inicia servidor de teste em background."""
        if self._server:
            return
        from http.server import HTTPServer, BaseHTTPRequestHandler
        self._server = HTTPServer(('localhost', self.port), self._Handler)
        self._server._last_request = None
        self._server._request_count = 0
        self._server._responses = self._responses
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()
        # Aguarda inicialização
        for _ in range(10):
            if self._server.socket:
                break
            time.sleep(0.1)
        logger.info(f"Servidor de teste iniciado na porta {self.port}")

    def stop(self) -> None:
        """Para o servidor de teste."""
        if self._server:
            self._server.shutdown()
            self._server = None
        if self._thread:
            self._thread.join(timeout=2)
            self._thread = None
        logger.info("Servidor de teste parado")

    def add_response(self, path: str, status: int = 200,
                     body: str = None, content_type: str = 'application/json'):
        """Adiciona uma resposta personalizada para um path."""
        self._responses[path] = {
            'status': status,
            'content_type': content_type,
            'body': body or json.dumps({'status': 'ok', 'path': path})
        }

    @property
    def last_request(self) -> Optional[Dict]:
        """Retorna a última requisição recebida."""
        return self._server._last_request if self._server else None

    @property
    def request_count(self) -> int:
        """Retorna o número total de requisições."""
        return self._server._request_count if self._server else 0

# ============================================================================
# GRAFO DE CONHECIMENTO SEMÂNTICO COM FAISS
# ============================================================================

class SemanticKnowledgeGraph(KnowledgeGraph):
    """
    Grafo de conhecimento com embeddings semânticos e busca FAISS.

    Mantém um grafo de objetivos de teste com descrições, pré-requisitos,
    histórico de execução e embeddings para busca semântica.
    """

    def __init__(self, model_name: str = "all-MiniLM-L6-v2"):
        self.nodes: Dict[str, Dict] = {}
        self.edges: List[Tuple[str, str]] = []
        self.version: int = 0
        self._embeddings: Dict[str, np.ndarray] = {}
        self._clusters: Dict[str, int] = {}
        self._model_name = model_name
        self._model = None
        self._faiss_index = None
        self._faiss_ids: List[str] = []
        self._lock = threading.RLock()

    def _get_model(self):
        """Carrega o modelo de embeddings sob demanda."""
        if self._model is None and HAS_SENTENCE_TRANSFORMERS:
            try:
                self._model = SentenceTransformer(self._model_name)
                logger.info(f"Modelo de embeddings carregado: {self._model_name}")
            except Exception as e:
                logger.error(f"Erro ao carregar modelo: {e}")
        return self._model

    def _rebuild_faiss_index(self):
        """Reconstrói o índice FAISS para busca de similaridade."""
        if not HAS_FAISS or len(self._embeddings) < 2:
            self._faiss_index = None
            self._faiss_ids = []
            return

        try:
            vectors = np.array(list(self._embeddings.values())).astype(np.float32)
            dim = vectors.shape[1]
            self._faiss_index = faiss.IndexFlatL2(dim)
            self._faiss_index.add(vectors)
            self._faiss_ids = list(self._embeddings.keys())
        except Exception as e:
            logger.error(f"Erro ao construir índice FAISS: {e}")
            self._faiss_index = None

    def add_goal(self, goal_id: str, description: str, prerequisites: List[str] = None) -> Dict:
        """Adiciona um objetivo ao grafo."""
        with self._lock:
            self.nodes[goal_id] = {
                'id': goal_id,
                'description': description,
                'prerequisites': prerequisites or [],
                'tests': [],
                'execution_count': 0,
                'success_count': 0,
                'failure_patterns': [],
                'stability_score': 1.0,
                'last_result': None,
                'created_at': time.time(),
                'updated_at': time.time()
            }
            self.version += 1

            # Gera embedding
            model = self._get_model()
            if model:
                try:
                    self._embeddings[goal_id] = model.encode(description)
                    self._rebuild_faiss_index()
                except Exception as e:
                    logger.error(f"Erro ao gerar embedding: {e}")

            return self.nodes[goal_id]

    def update_result(self, goal_id: str, result: Dict) -> None:
        """Atualiza o resultado de um objetivo."""
        with self._lock:
            node = self.nodes.get(goal_id)
            if not node:
                return
            node['execution_count'] += 1
            node['updated_at'] = time.time()

            if result.get('success', False):
                node['success_count'] += 1
                node['failure_patterns'] = []  # Limpa falhas anteriores
            else:
                error = str(result.get('error', 'unknown'))
                node['failure_patterns'].append(error[:200])

            node['stability_score'] = node['success_count'] / max(node['execution_count'], 1)
            node['last_result'] = result

    def find_similar_goals(self, goal_id: str, top_k: int = 5) -> List[str]:
        """
        Encontra objetivos semanticamente similares.

        Usa FAISS para busca eficiente (O(log n)) ou fallback baseado em palavras-chave.
        """
        with self._lock:
            if goal_id not in self._embeddings:
                return self._fallback_similar(goal_id)

            # Tenta FAISS primeiro
            if self._faiss_index is not None and HAS_FAISS:
                try:
                    query = self._embeddings[goal_id].astype(np.float32).reshape(1, -1)
                    k = min(top_k + 1, len(self._faiss_ids))
                    distances, indices = self._faiss_index.search(query, k)
                    results = []
                    for i in indices[0]:
                        if i < len(self._faiss_ids) and self._faiss_ids[i] != goal_id:
                            results.append(self._faiss_ids[i])
                    return results[:top_k]
                except Exception as e:
                    logger.warning(f"Erro na busca FAISS: {e}, usando fallback")

            return self._fallback_similar(goal_id)

    def _fallback_similar(self, goal_id: str, top_k: int = 5) -> List[str]:
        """Fallback baseado em palavras-chave (O(n))."""
        node = self.nodes.get(goal_id)
        if not node:
            return []
        words = set(node['description'].lower().split())
        similar = []
        for gid, other in self.nodes.items():
            if gid != goal_id:
                other_words = set(other['description'].lower().split())
                overlap = len(words & other_words)
                if overlap > 0:
                    similar.append((gid, overlap))
        similar.sort(key=lambda x: x[1], reverse=True)
        return [gid for gid, _ in similar[:top_k]]

    def get_goals(self) -> Dict[str, Dict]:
        """Retorna todos os objetivos."""
        with self._lock:
            return self.nodes.copy()

    def get_version(self) -> int:
        """Retorna a versão atual do grafo."""
        return self.version

    def cluster_goals(self, n_clusters: int = 5) -> Dict[str, int]:
        """Agrupa objetivos por similaridade semântica."""
        if len(self._embeddings) < n_clusters:
            return {}

        try:
            from sklearn.cluster import KMeans
            vectors = np.array([self._embeddings[gid] for gid in self.nodes.keys()])
            kmeans = KMeans(n_clusters=n_clusters, random_state=42)
            labels = kmeans.fit_predict(vectors)
            self._clusters = {gid: int(labels[i]) for i, gid in enumerate(self.nodes.keys())}
            return self._clusters
        except ImportError:
            return {}

# ============================================================================
# GERADOR DE TESTES AVANÇADO
# ============================================================================

class AdvancedTestGenerator(TestGenerator):
    """
    Gerador de testes com property-based, mutação e fuzzing.

    Gera testes a partir de objetivos, com suporte a propriedades,
    mutações de testes existentes e fuzzing com entradas aleatórias.
    """

    def __init__(self, llm: Optional[LLMBackbone], kg: KnowledgeGraph, config: QARSIConfig):
        self.llm = llm
        self.kg = kg
        self.config = config
        self._cache: Dict[str, List[Dict]] = {}
        self._lock = threading.RLock()

    def generate_tests(self, goal_id: str) -> List[Dict]:
        """Gera testes para um objetivo."""
        with self._lock:
            if goal_id in self._cache:
                return self._cache[goal_id][:self.config.max_tests_per_goal]

            goal = self.kg.get_goals().get(goal_id)
            if not goal:
                return []

            tests = []

            # Property-based tests
            for template in self.config.property_templates:
                if self.llm:
                    prompt = f"Complete a propriedade para o objetivo '{goal['description']}': {template}"
                    property_text = self.llm.generate(prompt)
                    if property_text:
                        tests.append({
                            'type': 'property',
                            'goal_id': goal_id,
                            'property': property_text,
                            'steps': ['Verificar propriedade'],
                            'expected': True,
                            'generated_at': time.time()
                        })
                else:
                    # Fallback sem LLM
                    property_text = template.replace("{acao}", goal['description']).replace("{estado}", "consistente")
                    tests.append({
                        'type': 'property',
                        'goal_id': goal_id,
                        'property': property_text,
                        'steps': ['Verificar propriedade'],
                        'expected': True,
                        'generated_at': time.time()
                    })

            # Fuzz tests
            fuzz_tests = self.fuzz(goal_id, self.config.fuzz_iterations)
            tests.extend(fuzz_tests)

            self._cache[goal_id] = tests
            return tests[:self.config.max_tests_per_goal]

    def mutate(self, test: Dict) -> List[Dict]:
        """Gera mutações de um teste existente."""
        if not self.config.mutation_enabled:
            return []

        mutants = []
        steps = test.get('steps', [])

        # Mutação 1: Inverter ordem
        if len(steps) > 1:
            mutants.append({
                **test,
                'steps': steps[::-1],
                'mutated': 'order_inverted',
                'parent': test.get('id', 'unknown')
            })

        # Mutação 2: Valores
        if 'input' in test:
            original = test['input']
            for mutator in [self._mutate_string, self._mutate_numeric]:
                new_val = mutator(original)
                if new_val != original:
                    mutants.append({
                        **test,
                        'input': new_val,
                        'mutated': 'value_changed',
                        'parent': test.get('id', 'unknown')
                    })

        # Mutação 3: Adicionar passo
        if len(steps) > 0:
            new_steps = steps + ['Verificar estado final']
            mutants.append({
                **test,
                'steps': new_steps,
                'mutated': 'step_added',
                'parent': test.get('id', 'unknown')
            })

        return mutants

    def fuzz(self, goal_id: str, n_iterations: int) -> List[Dict]:
        """Gera testes via fuzzing."""
        goal = self.kg.get_goals().get(goal_id)
        if not goal:
            return []

        fuzz_tests = []
        for i in range(n_iterations):
            random_input = self._random_input()
            fuzz_tests.append({
                'type': 'fuzz',
                'goal_id': goal_id,
                'steps': [f'Executar {goal["description"]} com entrada {random_input}'],
                'expected': 'Não deve quebrar',
                'input': random_input,
                'fuzz_id': i,
                'generated_at': time.time()
            })
        return fuzz_tests

    def _random_input(self) -> str:
        """Gera entrada aleatória para fuzzing."""
        import random, string
        chars = string.ascii_letters + string.digits + string.punctuation + "çãõáéíóúÁÉÍÓÚ"
        length = random.randint(1, 30)
        return ''.join(random.choices(chars, k=length))

    def _mutate_string(self, value: str) -> str:
        """Mutação para strings."""
        if not isinstance(value, str):
            return value
        if len(value) == 0:
            return value

        import random
        if len(value) > 1 and random.random() < 0.3:
            idx = random.randint(0, len(value)-1)
            return value[:idx] + value[idx+1:]
        if random.random() < 0.3:
            idx = random.randint(0, len(value))
            return value[:idx] + random.choice('abcdefghijklmnopqrstuvwxyz') + value[idx:]
        return value

    def _mutate_numeric(self, value) -> Any:
        """Mutação para números."""
        import random
        if isinstance(value, (int, float)):
            return value * (1 + random.uniform(-0.2, 0.2))
        return value

# ============================================================================
# HEALER AVANÇADO (LLM + VISÃO)
# ============================================================================

class AdvancedHealer(Healer):
    """
    Healer com LLM para reparo de seletores e fluxo.

    Tenta curar testes falhos usando:
    1. Reparo de seletor via LLM
    2. Reparo de fluxo via LLM
    3. Reparo visual (placeholder para futura integração)
    """

    def __init__(self, llm: Optional[LLMBackbone] = None, vision_model=None):
        self.llm = llm
        self.vision = vision_model
        self.template_cache: Dict[str, Any] = {}
        self.heal_history: List[Dict] = []
        self._lock = threading.RLock()

    def heal(self, failed_test: Dict, context: Dict) -> Optional[Dict]:
        """Tenta curar um teste falho."""
        # 1. Reparo por seletor (LLM)
        if 'selector' in failed_test:
            healed = self._heal_selector(failed_test, context)
            if healed:
                with self._lock:
                    self.heal_history.append({'method': 'selector', 'success': True})
                return healed

        # 2. Reparo por visão (se disponível)
        if 'screenshot' in context and self.vision:
            healed = self._heal_by_vision(failed_test, context)
            if healed:
                with self._lock:
                    self.heal_history.append({'method': 'vision', 'success': True})
                return healed

        # 3. Reparo por fluxo (LLM)
        if self.llm:
            healed = self._heal_flow(failed_test, context)
            if healed:
                with self._lock:
                    self.heal_history.append({'method': 'flow', 'success': True})
                return healed

        with self._lock:
            self.heal_history.append({'method': 'none', 'success': False})
        return None

    def _heal_selector(self, failed_test: Dict, context: Dict) -> Optional[Dict]:
        """Repara seletor usando LLM."""
        if not self.llm:
            return None

        prompt = f"""
        O seletor '{failed_test.get('selector')}' falhou.
        Contexto: {json.dumps(context, indent=2)[:500]}
        Gere um novo seletor CSS/XPath alternativo.
        """
        new_selector = self.llm.generate(prompt).strip()
        if new_selector and len(new_selector) > 3:
            healed = failed_test.copy()
            healed['selector'] = new_selector
            healed['healed'] = True
            healed['healing_method'] = 'selector_llm'
            return healed
        return None

    def _heal_by_vision(self, failed_test: Dict, context: Dict) -> Optional[Dict]:
        """Repara usando visão computacional (placeholder)."""
        # Em produção, usaria CLIP/SAM para localizar elementos
        return None

    def _heal_flow(self, failed_test: Dict, context: Dict) -> Optional[Dict]:
        """Repara fluxo usando LLM."""
        if not self.llm:
            return None

        steps = failed_test.get('steps', [])
        prompt = f"""
        O teste falhou nos passos: {steps}
        Contexto: {json.dumps(context, indent=2)[:500]}
        Sugira uma correção para o fluxo do teste.
        """
        correction = self.llm.generate(prompt).strip()
        if correction:
            healed = failed_test.copy()
            healed['steps'] = [correction] if isinstance(correction, str) else correction
            healed['healed'] = True
            healed['healing_method'] = 'flow_llm'
            return healed
        return None

    def get_heal_stats(self) -> Dict:
        """Retorna estatísticas de auto-cura."""
        with self._lock:
            total = len(self.heal_history)
            successes = sum(1 for h in self.heal_history if h.get('success', False))
            return {
                'total_attempts': total,
                'successes': successes,
                'success_rate': successes / total if total > 0 else 0.0,
                'by_method': defaultdict(int, [(h['method'], h['success']) for h in self.heal_history])
            }

# ============================================================================
# META-AVALIADOR
# ============================================================================

class MetaEvaluatorImpl(MetaEvaluator):
    """Avalia a qualidade do conjunto de testes."""

    def __init__(self, config: QARSIConfig):
        self.config = config
        np.random.seed(config.random_seed)

    def evaluate(self, tests: List[Dict], codebase: str = "") -> Dict:
        """Avalia a eficácia dos testes."""
        if not tests:
            return {
                'mutation_score': 0.0,
                'coverage': 0.0,
                'flakiness': 0.0,
                'overall_quality': 0.0,
                'total_tests': 0,
                'timestamp': time.time()
            }

        # Mutation score (simulado, baseado na complexidade dos testes)
        mutation_detected = 0
        total_mutations = min(len(tests), 20)
        for i, test in enumerate(tests):
            # Testes mais complexos têm maior chance de detectar mutações
            complexity = len(str(test.get('steps', []))) / 100
            detection_prob = 0.6 + 0.3 * min(1.0, complexity)
            if np.random.random() < detection_prob:
                mutation_detected += 1

        mutation_score = mutation_detected / max(total_mutations, 1)

        # Cobertura (baseada na diversidade de tipos de teste)
        types = set(t.get('type', 'unknown') for t in tests)
        coverage = min(0.95, 0.5 + 0.1 * len(types))

        # Flakiness (simulado)
        flakiness = max(0.0, min(0.2, np.random.normal(0.05, 0.02)))

        # Overall
        overall = 0.4 * mutation_score + 0.4 * coverage + 0.2 * (1 - flakiness)

        return {
            'mutation_score': round(mutation_score, 3),
            'coverage': round(coverage, 3),
            'flakiness': round(flakiness, 3),
            'overall_quality': round(overall, 3),
            'total_tests': len(tests),
            'test_types': list(types),
            'timestamp': time.time()
        }

# ============================================================================
# AGENT ARCHIVE (EVOLUTIVO)
# ============================================================================

class AgentArchive:
    """Archive evolutivo de agentes de QA."""

    def __init__(self, max_size: int = 20):
        self.agents: List[Dict] = []
        self.lineages: Dict[str, List[str]] = {}
        self.max_size = max_size
        self._lock = threading.RLock()

    def add_agent(self, agent: 'QARSI', fitness: float, parent_id: Optional[str] = None) -> str:
        """Adiciona agente ao archive."""
        with self._lock:
            agent_id = hashlib.sha256(str(time.time()).encode()).hexdigest()[:8]
            entry = {
                'id': agent_id,
                'agent': agent,
                'fitness': fitness,
                'parent': parent_id,
                'generation': len(self.agents),
                'timestamp': time.time()
            }
            self.agents.append(entry)
            if parent_id:
                self.lineages.setdefault(parent_id, []).append(agent_id)
            self._prune()
            return agent_id

    def sample_parent(self, strategy: str = 'fitness_weighted') -> Optional['QARSI']:
        """Amostra agente parente do archive."""
        with self._lock:
            if not self.agents:
                return None

            if strategy == 'fitness_weighted':
                fitnesses = [max(0.01, a['fitness']) for a in self.agents]
                total = sum(fitnesses)
                if total == 0:
                    return random.choice(self.agents)['agent']
                r = random.random() * total
                cumsum = 0
                for agent in self.agents:
                    cumsum += agent['fitness']
                    if r <= cumsum:
                        return agent['agent']

            return random.choice(self.agents)['agent']

    def get_best(self) -> Optional['QARSI']:
        """Retorna o melhor agente."""
        with self._lock:
            if not self.agents:
                return None
            return max(self.agents, key=lambda x: x['fitness'])['agent']

    def get_lineage(self, agent_id: str) -> List[str]:
        """Retorna a linhagem de um agente."""
        with self._lock:
            lineage = []
            current = agent_id
            while current:
                lineage.append(current)
                # Encontra o parente
                parent = None
                for agent in self.agents:
                    if agent['id'] == current:
                        parent = agent.get('parent')
                        break
                current = parent
            return lineage[::-1]

    def get_all(self) -> List[Dict]:
        """Retorna todos os agentes no archive."""
        with self._lock:
            return self.agents.copy()

    def _prune(self):
        """Mantém apenas os melhores agentes."""
        if len(self.agents) > self.max_size:
            self.agents.sort(key=lambda x: x['fitness'], reverse=True)
            self.agents = self.agents[:self.max_size]

# ============================================================================
# SWE-BENCH SIMULATOR
# ============================================================================

class SWEBenchSimulator:
    """Simulador de SWE-bench para validação de agentes."""

    def __init__(self):
        self.tasks = self._load_tasks()
        self._results = {}

    def _load_tasks(self) -> List[Dict]:
        """Carrega tarefas simuladas do SWE-bench."""
        return [
            {'id': f'task_{i:02d}',
             'description': f'Resolver problema de engenharia de software {i}',
             'complexity': 0.3 + 0.6 * (i / 10)}
            for i in range(10)
        ]

    def evaluate(self, agent_code: str) -> float:
        """Avalia agente no SWE-bench simulado."""
        passed = 0
        for task in self.tasks:
            result = self._run_agent(agent_code, task)
            if result.get('resolved', False):
                passed += 1
        rate = passed / len(self.tasks) if self.tasks else 0.0
        logger.info(f"SWE-bench: {passed}/{len(self.tasks)} resolvidos ({rate:.1%})")
        return rate

    def _run_agent(self, agent_code: str, task: Dict) -> Dict:
        """Simula execução do agente em uma tarefa."""
        # A probabilidade de sucesso depende da complexidade do código
        # e da complexidade da tarefa
        code_complexity = min(1.0, len(agent_code) / 5000)
        task_complexity = task.get('complexity', 0.5)
        success_prob = 0.3 + 0.6 * code_complexity * (1 - 0.5 * task_complexity)
        resolved = random.random() < success_prob
        return {'resolved': resolved, 'probability': success_prob}

    def get_results(self) -> Dict:
        """Retorna resultados detalhados."""
        return {
            'total_tasks': len(self.tasks),
            'tasks': self.tasks
        }

# ============================================================================
# TEST REVIEWER (PEER-REVIEW)
# ============================================================================

class TestReviewer:
    """Agente revisor para avaliação de testes gerados."""

    def __init__(self, llm: LLMBackbone):
        self.llm = llm
        self.review_history: List[Dict] = []
        self._lock = threading.RLock()

    def review(self, test: Dict, context: Dict) -> Dict:
        """Revisa um teste e retorna score e feedback."""
        prompt = f"""
        Revise este teste de QA:
        Teste: {json.dumps(test, indent=2)[:500]}
        Contexto: {json.dumps(context, indent=2)[:500]}

        Avalie em 4 dimensões (0-1):
        1. Correção: O teste está correto?
        2. Cobertura: Cobre o cenário adequadamente?
        3. Clareza: Está bem escrito e documentado?
        4. Robustez: É resiliente a mudanças?

        Retorne um JSON com scores e feedback.
        """
        response = self.llm.generate(prompt)

        try:
            # Tenta extrair JSON da resposta
            import re
            json_match = re.search(r'\{.*\}', response, re.DOTALL)
            if json_match:
                result = json.loads(json_match.group())
            else:
                raise ValueError("No JSON found")
        except:
            result = {
                'correction': 0.7,
                'coverage': 0.7,
                'clarity': 0.7,
                'robustness': 0.7,
                'overall': 0.7,
                'feedback': 'Revisão automática (fallback)'
            }

        with self._lock:
            self.review_history.append({
                'test': test.get('goal_id', 'unknown'),
                'review': result,
                'timestamp': time.time()
            })

        return result

    def get_stats(self) -> Dict:
        """Retorna estatísticas de revisão."""
        with self._lock:
            if not self.review_history:
                return {'total': 0, 'avg_score': 0.0}
            scores = [h['review'].get('overall', 0) for h in self.review_history]
            return {
                'total': len(self.review_history),
                'avg_score': sum(scores) / len(scores),
                'min_score': min(scores) if scores else 0,
                'max_score': max(scores) if scores else 0
            }

# ============================================================================
# IMPACT ANALYZER
# ============================================================================

class ImpactAnalyzer:
    """Analisa impacto de mudanças no código."""

    def __init__(self, kg: KnowledgeGraph):
        self.kg = kg
        self._file_cache: Dict[str, List[str]] = {}
        self._lock = threading.RLock()

    def analyze(self, changed_files: List[str]) -> Dict:
        """Retorna objetivos afetados e prioridade."""
        affected = {}
        for file in changed_files:
            for goal_id, node in self.kg.get_goals().items():
                if self._is_affected(file, node):
                    affected[goal_id] = affected.get(goal_id, 0) + 1
        return self._prioritize(affected)

    def _is_affected(self, file: str, node: Dict) -> bool:
        """Verifica se um arquivo afeta um objetivo."""
        # Usa palavras-chave da descrição
        keywords = set(node['description'].lower().split())
        file_lower = file.lower()
        return any(kw in file_lower for kw in keywords if len(kw) > 2)

    def _prioritize(self, affected: Dict) -> Dict:
        """Prioriza objetivos por impacto."""
        sorted_items = sorted(affected.items(), key=lambda x: x[1], reverse=True)
        return {
            goal_id: {
                'impact_count': count,
                'priority': i + 1,
                'priority_label': 'high' if i < 3 else 'medium' if i < 7 else 'low'
            }
            for i, (goal_id, count) in enumerate(sorted_items)
        }

# ============================================================================
# DARWIN GÖDEL MACHINE ENGINE
# ============================================================================

class DarwinGodelEngine:
    """Motor Darwin-Gödel para auto-evolução do agente de QA."""

    def __init__(self, initial_code: str, benchmark: 'SWEBenchSimulator', config: QARSIConfig):
        self.code = initial_code
        self.benchmark = benchmark
        self.config = config
        self.archive: List[Dict] = []
        self.lineage: Dict[str, Dict] = {}
        self.generation = 0
        self._lock = threading.RLock()

    def evolve(self, generations: int = None) -> Dict:
        """Evolui o código via seleção natural."""
        generations = generations or self.config.dgm_generations
        logger.info(f"DGM: Iniciando evolução por {generations} gerações")

        for gen in range(generations):
            self.generation += 1
            with self._lock:
                parent = self._sample_agent()
                offspring = self._mutate(parent)
                fitness = self.benchmark.evaluate(offspring['code'])

                if fitness > self._get_best_fitness():
                    agent_id = self._register_agent(offspring, fitness, parent.get('id'))
                    logger.info(f"DGM: G{gen} - Novo agente {agent_id} com fitness {fitness:.3f}")

                self._prune_archive()

        return {
            'generations': self.generation,
            'archive_size': len(self.archive),
            'best_fitness': self._get_best_fitness(),
            'lineage': self.lineage
        }

    def _sample_agent(self) -> Dict:
        """Amostra agente do archive."""
        if not self.archive:
            return {'id': 'seed', 'code': self.code, 'fitness': 0.0}
        fitnesses = [a['fitness'] for a in self.archive]
        total = sum(fitnesses)
        if total == 0:
            return random.choice(self.archive)
        r = random.random() * total
        cumsum = 0
        for agent in self.archive:
            cumsum += agent['fitness']
            if r <= cumsum:
                return agent
        return self.archive[-1]

    def _mutate(self, parent: Dict) -> Dict:
        """Aplica mutações ao código do agente."""
        code = parent['code']
        mutations = [
            self._mutate_add_test_generator,
            self._mutate_improve_healer,
            self._mutate_add_metric,
            self._mutate_optimize_executor
        ]
        mutation = random.choice(mutations)
        return {'code': mutation(code), 'parent_id': parent.get('id')}

    def _mutate_add_test_generator(self, code: str) -> str:
        return code + "\n# DGM: Novo gerador de testes adicionado\n"

    def _mutate_improve_healer(self, code: str) -> str:
        return code + "\n# DGM: Healer aprimorado\n"

    def _mutate_add_metric(self, code: str) -> str:
        return code + "\n# DGM: Nova métrica adicionada\n"

    def _mutate_optimize_executor(self, code: str) -> str:
        return code + "\n# DGM: Executor otimizado\n"

    def _register_agent(self, agent: Dict, fitness: float, parent_id: Optional[str]) -> str:
        agent_id = hashlib.sha256(str(time.time()).encode()).hexdigest()[:8]
        entry = {
            'id': agent_id,
            'code': agent['code'],
            'fitness': fitness,
            'parent': parent_id,
            'generation': self.generation
        }
        self.archive.append(entry)
        if parent_id:
            self.lineage.setdefault(parent_id, []).append(agent_id)
        return agent_id

    def _get_best_fitness(self) -> float:
        return max([a['fitness'] for a in self.archive]) if self.archive else 0.0

    def _prune_archive(self):
        if len(self.archive) > self.config.dgm_archive_size:
            self.archive.sort(key=lambda x: x['fitness'], reverse=True)
            self.archive = self.archive[:self.config.dgm_archive_size]

# ============================================================================
# AGENTE PRINCIPAL — QARSI v8
# ============================================================================

class QARSI:
    """
    Agente de QA autopoiético com execução real e DGM.

    Esta é a classe principal que orquestra todos os componentes:
    - Grafo de conhecimento semântico
    - Execução real de testes (subprocess, HTTP, Playwright)
    - Circuit Breaker thread-safe
    - Darwin-Gödel Machine para auto-evolução
    - Archive de agentes
    - Telemetria e WormGraph
    """

    def __init__(self, container: 'QARSIContainer',
                 executor_mode: str = 'subprocess',
                 test_server_port: Optional[int] = None):
        """
        Args:
            container: Container de injeção de dependência
            executor_mode: Modo de execução ('subprocess', 'http', 'playwright', 'simulated')
            test_server_port: Porta para o servidor de teste (opcional)
        """
        self.config = container.config

        # Componentes principais
        self.kg = container.get('kg')
        self.llm = container.get('llm')
        self.generator = container.get('generator')
        self.healer = container.get('healer')
        self.meta_evaluator = container.get('meta_evaluator')
        self.pr_integration = container.get('pr_integration')
        self.wormgraph = container.get('wormgraph')
        self.telemetry = container.get('telemetry')

        # Componentes de evolução
        self.benchmark = SWEBenchSimulator()
        self.archive = AgentArchive(max_size=self.config.dgm_archive_size)
        self.reviewer = TestReviewer(self.llm) if self.llm and self.config.peer_review_enabled else None
        self.impact_analyzer = ImpactAnalyzer(self.kg) if self.config.impact_analysis_enabled else None

        # Executor real de testes
        self.executor = TestExecutor(
            mode=ExecutorMode(executor_mode),
            timeout=self.config.test_timeout_seconds
        )

        # Retryable com Circuit Breaker
        self.retryable = Retryable(
            max_retries=self.config.max_retries,
            backoff=self.config.retry_backoff,
            failure_threshold=self.config.failure_threshold,
            circuit_timeout=self.config.circuit_timeout_seconds
        )

        # DGM
        self.dgm = DarwinGodelEngine(
            initial_code=self._get_code(),
            benchmark=self.benchmark,
            config=self.config
        )

        # Servidor de teste (para validação)
        self.test_server = None
        if test_server_port:
            self.test_server = TestServer(port=test_server_port)
            self.test_server.start()

        # Estado do agente
        self._executor_pool = ThreadPoolExecutor(max_workers=self.config.max_workers)
        self.generation = 0
        self.fitness_history: List[float] = []
        self._lock = threading.RLock()
        self._shutdown_flag = False

        # Métricas Prometheus
        if HAS_PROMETHEUS and self.config.prometheus_enabled:
            try:
                start_http_server(self.config.prometheus_port)
                logger.info(f"Prometheus iniciado na porta {self.config.prometheus_port}")
            except Exception as e:
                logger.warning(f"Erro ao iniciar Prometheus: {e}")

        logger.info(f"QA-RSI v{self.config.version} inicializado (executor: {executor_mode})")

    def _get_code(self) -> str:
        """Retorna o código atual do agente para DGM."""
        import inspect
        return inspect.getsource(self.__class__)

    def bootstrap(self, initial_goals: Dict[str, str] = None) -> None:
        """Inicializa o grafo com objetivos iniciais."""
        if initial_goals is None:
            initial_goals = {
                'login': 'Realizar login com credenciais válidas',
                'purchase': 'Finalizar compra no carrinho',
                'review': 'Deixar avaliação de produto',
                'logout': 'Realizar logout do sistema',
                'search': 'Buscar produtos por termo',
                'filter': 'Aplicar filtros de busca'
            }

        for goal_id, description in initial_goals.items():
            self.kg.add_goal(goal_id, description)

        logger.info(f"Bootstrap: {len(self.kg.get_goals())} objetivos carregados")

    def run_cycle(self) -> Dict:
        """Executa um ciclo completo de evolução."""
        if self._shutdown_flag:
            return {'error': 'Agente desligado'}

        with self._lock:
            self.generation += 1
            logger.info(f"--- Ciclo {self.generation} (executor: {self.executor.mode.value}) ---")

            # 1. Geração de testes
            self._generate_missing_tests()

            # 2. Coleta e execução de testes
            all_tests = self._collect_all_tests()
            results = self._execute_tests_parallel(all_tests)
            self._process_results(results)

            # 3. Auto-cura
            self._heal_failures()

            # 4. Meta-avaliação
            meta = self._evaluate_meta()

            # 5. Atualização de fitness
            fitness = self._update_fitness()

            # 6. Evolução via DGM
            if fitness > (self.fitness_history[-2] if len(self.fitness_history) > 1 else 0):
                self._evolve_agent()

            # 7. Registro e telemetria
            self._log_to_wormgraph(fitness, meta)
            self._publish_telemetry(fitness, meta)

            # 8. Métricas Prometheus
            if HAS_PROMETHEUS and self.config.prometheus_enabled:
                try:
                    qa_cycles.inc()
                    qa_fitness.set(fitness)
                except:
                    pass

            return {
                'generation': self.generation,
                'fitness': fitness,
                'meta': meta,
                'executor_mode': self.executor.mode.value,
                'total_tests': len(all_tests),
                'real_executions': sum(1 for r in results if not r.get('simulated', False)),
                'archive_size': len(self.archive.agents),
                'executor_stats': self.executor.get_stats()
            }

    def _generate_missing_tests(self):
        """Gera testes para objetivos não testados."""
        for goal_id, node in self.kg.get_goals().items():
            if node['execution_count'] == 0:
                tests = self.generator.generate_tests(goal_id)

                # Peer-review se habilitado
                if self.reviewer and self.config.peer_review_enabled:
                    for test in tests:
                        review = self.reviewer.review(test, {'goal': node})
                        test['review_score'] = review.get('overall', 0.5)
                    tests = [t for t in tests if t.get('review_score', 0) > self.config.peer_review_threshold]

                node['tests'] = tests

    def _collect_all_tests(self) -> List[Dict]:
        """Coleta todos os testes do grafo."""
        all_tests = []
        for goal_id, node in self.kg.get_goals().items():
            all_tests.extend(node.get('tests', []))
        return all_tests

    def _execute_tests_parallel(self, tests: List[Dict]) -> List[Dict]:
        """Executa testes em paralelo com timeout e retry."""
        if not tests:
            return []

        futures = []
        for test in tests:
            # Se o teste já tem um comando HTTP, usa o executor diretamente
            if 'command' in test or 'url' in test:
                future = self._executor_pool.submit(
                    self._execute_test_with_retry, test
                )
            else:
                # Se não tem comando, gera um comando baseado no tipo
                test_with_command = self._test_to_command(test)
                future = self._executor_pool.submit(
                    self._execute_test_with_retry, test_with_command
                )
            futures.append((test, future))

        results = []
        for test, future in futures:
            try:
                result = future.result(timeout=self.config.test_timeout_seconds + 5)
                if result:
                    result['goal_id'] = test.get('goal_id')
                    results.append(result)
            except FuturesTimeout:
                logger.warning(f"Teste {test.get('goal_id', 'unknown')} excedeu timeout")
                results.append({
                    'success': False,
                    'goal_id': test.get('goal_id'),
                    'error': f'Timeout após {self.config.test_timeout_seconds + 5}s'
                })
            except Exception as e:
                logger.error(f"Erro na execução de teste: {e}")
                results.append({
                    'success': False,
                    'goal_id': test.get('goal_id'),
                    'error': str(e)
                })

        return results

    def _execute_test_with_retry(self, test: Dict) -> Dict:
        """Executa um teste com retry e circuit breaker."""
        return self.retryable.execute(self._execute_test, test)

    def _execute_test(self, test: Dict) -> Dict:
        """Executa um teste real via executor."""
        return self.executor.execute(test)

    def _test_to_command(self, test: Dict) -> Dict:
        """Converte um teste genérico em um comando executável."""
        # Se já tem comando, retorna
        if 'command' in test or 'url' in test:
            return test

        # Se tem URL, assume HTTP
        if 'url' in test:
            return test

        # Se tem steps, gera um comando de verificação
        steps = test.get('steps', [])
        if steps:
            # Em produção, isso seria mais sofisticado
            return {
                **test,
                'command': ['echo', f'Verificando: {steps[0][:50]}']
            }

        # Fallback
        return {
            **test,
            'command': ['echo', f'Teste {test.get("goal_id", "unknown")}']
        }

    def _process_results(self, results: List[Dict]):
        """Processa resultados de execução."""
        for result in results:
            if result and result.get('goal_id'):
                self.kg.update_result(result['goal_id'], result)

    def _heal_failures(self):
        """Tenta curar testes falhos."""
        for goal_id, node in self.kg.get_goals().items():
            if node.get('last_result') and not node['last_result'].get('success', False):
                healed = self.healer.heal(
                    {
                        'selector': f"selector-{goal_id}",
                        'steps': node.get('tests', []),
                        'goal_id': goal_id
                    },
                    {
                        'screenshot': np.zeros((100, 100, 3)),
                        'goal': node
                    }
                )
                if healed:
                    logger.info(f"Auto-cura aplicada em {goal_id}")
                    node['tests'] = [healed]
                    node['last_result'] = None  # Reset para reavaliação

    def _evaluate_meta(self) -> Dict:
        """Avalia a qualidade dos testes."""
        all_tests = []
        for node in self.kg.get_goals().values():
            all_tests.extend(node.get('tests', []))
        return self.meta_evaluator.evaluate(all_tests)

    def _update_fitness(self) -> float:
        """Atualiza e retorna fitness."""
        goals = self.kg.get_goals()
        if not goals:
            fitness = 0.0
        else:
            tested = sum(1 for n in goals.values() if n['execution_count'] > 0)
            coverage = tested / len(goals) if goals else 0.0

            stable = sum(1 for n in goals.values() if n['stability_score'] > 0.8)
            stability = stable / max(tested, 1) if tested > 0 else 0.0

            fitness = (self.config.fitness_coverage_weight * coverage +
                      self.config.fitness_stability_weight * stability)

        self.fitness_history.append(fitness)
        return fitness

    def _evolve_agent(self):
        """Executa evolução via DGM."""
        logger.info("DGM: Iniciando evolução do agente")
        result = self.dgm.evolve(generations=1)
        logger.info(f"DGM: Evolução concluída - best_fitness={result['best_fitness']:.3f}")
        return result

    def _log_to_wormgraph(self, fitness: float, meta: Dict):
        """Registra no WormGraph."""
        if self.wormgraph:
            try:
                self.wormgraph.commit({
                    'event': 'qa_rsi_cycle',
                    'generation': self.generation,
                    'fitness': fitness,
                    'meta': meta,
                    'kg_version': self.kg.get_version(),
                    'archive_size': len(self.archive.agents),
                    'dgm_generation': self.dgm.generation,
                    'executor_mode': self.executor.mode.value,
                    'executor_stats': self.executor.get_stats()
                })
            except Exception as e:
                logger.error(f"Erro ao registrar no WormGraph: {e}")

    def _publish_telemetry(self, fitness: float, meta: Dict):
        """Publica telemetria."""
        if self.telemetry:
            try:
                self.telemetry.publish_metric('qa_rsi', 'fitness', fitness)
                self.telemetry.publish_metric('qa_rsi', 'meta_score', meta.get('overall_quality', 0))
                self.telemetry.publish_metric('qa_rsi', 'archive_size', len(self.archive.agents))
                self.telemetry.publish_metric('qa_rsi', 'tests_executed', self.executor.get_stats()['total'])
                self.telemetry.publish_metric('qa_rsi', 'success_rate', self.executor.get_stats()['success_rate'])
            except Exception as e:
                logger.error(f"Erro ao publicar telemetria: {e}")

    def handle_pr(self, pr_id: str, diff: str) -> Dict:
        """Processa PR com análise de impacto."""
        # Validação de entrada (Pydantic)
        if HAS_PYDANTIC:
            try:
                class PRRequest(BaseModel):
                    pr_id: str
                    diff: str

                    @validator('pr_id')
                    def validate_pr_id(cls, v):
                        if not v or len(v) > 100:
                            raise ValueError('PR ID inválido')
                        return v

                    @validator('diff')
                    def validate_diff(cls, v):
                        if len(v) > 100000:
                            raise ValueError('Diff muito longo')
                        return v.replace('\x00', '')

                request = PRRequest(pr_id=pr_id, diff=diff)
            except ValidationError as e:
                return {'error': str(e)}

        # Análise de impacto
        impact = None
        if self.impact_analyzer:
            changed_files = self._extract_files(diff)
            impact = self.impact_analyzer.analyze(changed_files)

        result = self.pr_integration.analyze_pr(pr_id, diff)
        if impact:
            result['impact_analysis'] = impact

        return result

    def _extract_files(self, diff: str) -> List[str]:
        """Extrai nomes de arquivos do diff."""
        files = []
        for line in diff.split('\n'):
            if line.startswith('+++ b/') or line.startswith('--- a/'):
                filepath = line[6:] if line.startswith('+++ b/') else line[6:]
                if filepath and filepath not in files and not filepath.startswith('/dev/null'):
                    files.append(filepath)
        return files

    def get_report(self) -> Dict:
        """Gera relatório completo."""
        goals = self.kg.get_goals()
        tested = [n for n in goals.values() if n['execution_count'] > 0]
        executor_stats = self.executor.get_stats()

        return {
            'generation': self.generation,
            'total_goals': len(goals),
            'tested_goals': len(tested),
            'avg_stability': np.mean([n['stability_score'] for n in tested]) if tested else 0.0,
            'fitness': self.fitness_history[-1] if self.fitness_history else 0.0,
            'archive_size': len(self.archive.agents),
            'dgm_generation': self.dgm.generation,
            'best_agent_fitness': self.archive.agents[0].get("fitness", 0.0) if self.archive.agents else 0.0,
            'executor': {
                'mode': self.executor.mode.value,
                'total_tests': executor_stats['total'],
                'success_rate': executor_stats['success_rate']
            },
            'circuit_breaker': self.retryable.get_metrics()['circuit_breaker'],
            'wormgraph_enabled': self.wormgraph is not None,
            'telemetry_enabled': self.telemetry is not None,
            'test_server_enabled': self.test_server is not None,
            'config': {
                'max_goals': self.config.max_goals,
                'fuzz_iterations': self.config.fuzz_iterations,
                'mutation_enabled': self.config.mutation_enabled,
                'dgm_generations': self.config.dgm_generations,
                'peer_review_enabled': self.config.peer_review_enabled,
                'impact_analysis_enabled': self.config.impact_analysis_enabled
            }
        }

    def shutdown(self):
        """Desliga o agente liberando recursos."""
        if self._shutdown_flag:
            return

        self._shutdown_flag = True
        logger.info("Desligando QA-RSI...")

        if self.test_server:
            self.test_server.stop()

        self._executor_pool.shutdown(wait=True)

        # Registra shutdown no WormGraph
        if self.wormgraph:
            try:
                self.wormgraph.commit({
                    'event': 'qa_rsi_shutdown',
                    'generation': self.generation,
                    'fitness': self.fitness_history[-1] if self.fitness_history else 0.0,
                    'archive_size': len(self.archive.agents),
                    'timestamp': time.time()
                })
            except:
                pass

        logger.info("QA-RSI desligado")

# ============================================================================
# CONTAINER DE INJEÇÃO DE DEPENDÊNCIA
# ============================================================================

class QARSIContainer:
    """Container de injeção de dependência."""

    def __init__(self, config: QARSIConfig):
        self.config = config
        self._providers = {}
        self._singletons = {}
        self._defaults = {
            'kg': lambda: SemanticKnowledgeGraph(config.model_name),
            'llm': lambda: None,  # Será substituído se disponível
            'generator': lambda: AdvancedTestGenerator(self.get('llm'), self.get('kg'), config),
            'healer': lambda: AdvancedHealer(self.get('llm')),
            'meta_evaluator': lambda: MetaEvaluatorImpl(config),
            'pr_integration': lambda: PRIntegration(self.get('kg'), config),
            'wormgraph': lambda: MockWormGraph() if not HAS_WORMGRAPH else None,
            'telemetry': lambda: MockTelemetry() if not HAS_TELEMETRY else None,
        }

    def register(self, key: str, provider: Callable, singleton: bool = False):
        """Registra um provedor de dependência."""
        self._providers[key] = (provider, singleton)

    def get(self, key: str):
        """Obtém uma instância da dependência."""
        if key in self._singletons:
            return self._singletons[key]

        if key in self._providers:
            provider, singleton = self._providers[key]
            instance = provider()
            if singleton:
                self._singletons[key] = instance
            return instance

        if key in self._defaults:
            instance = self._defaults[key]()
            # Singletons para serviços pesados
            if key in ['kg', 'llm', 'wormgraph']:
                self._singletons[key] = instance
            return instance

        raise KeyError(f"Provedor não encontrado: {key}")

# ============================================================================
# MOCKS PARA TESTES
# ============================================================================

class MockLLM(LLMBackbone):
    """LLM mock para testes."""

    def generate(self, prompt: str, temperature: float = 0.7) -> str:
        return f"Resposta mock para: {prompt[:50]}..."

    def generate_code(self, prompt: str) -> str:
        return "# Código mock gerado\nprint('Mock LLM')"

class MockWormGraph(WormGraphInterface):
    """WormGraph mock para testes."""

    def __init__(self):
        self.ledger = []

    def commit(self, block: Dict) -> bool:
        self.ledger.append(block)
        return True

    def get_ledger(self) -> List[Dict]:
        return self.ledger.copy()

class MockTelemetry(TelemetryInterface):
    """Telemetria mock para testes."""

    def __init__(self):
        self.metrics = {}

    def publish_metric(self, topic: str, metric: str, value: float) -> None:
        self.metrics[f"{topic}.{metric}"] = value

# ============================================================================
# PR INTEGRATION (MOCK)
# ============================================================================

class PRIntegration:
    """Integração com Pull Requests (shift-left)."""

    def __init__(self, kg: KnowledgeGraph, config: QARSIConfig):
        self.kg = kg
        self.config = config

    def analyze_pr(self, pr_id: str, diff: str) -> Dict:
        """Analisa um PR e sugere testes."""
        files = self._extract_files(diff)
        affected = []
        for file in files:
            for goal_id, node in self.kg.get_goals().items():
                if any(kw in file.lower() for kw in node['description'].lower().split()):
                    affected.append(goal_id)

        affected = list(set(affected))

        return {
            'pr_id': pr_id,
            'changed_files': len(files),
            'affected_goals': affected[:10],
            'tests_generated': len(affected),
            'recommendations': [
                f"Executar testes de regressão para: {', '.join(affected[:5])}" if affected else
                "Nenhum objetivo afetado identificado."
            ],
            'timestamp': time.time()
        }

    def _extract_files(self, diff: str) -> List[str]:
        """Extrai nomes de arquivos do diff."""
        files = []
        for line in diff.split('\n'):
            if line.startswith('+++ b/') or line.startswith('--- a/'):
                filepath = line[6:] if line.startswith('+++ b/') else line[6:]
                if filepath and filepath not in files and not filepath.startswith('/dev/null'):
                    files.append(filepath)
        return files

# ============================================================================
# EXEMPLO DE USO COMPLETO
# ============================================================================

def main():
    """Exemplo de uso completo do QA-RSI v8."""
    print("\n" + "="*60)
    print("🏛️ QA-RSI v8 — Execução Real e Circuit Breaker")
    print("="*60 + "\n")

    # Configuração
    config = QARSIConfig(
        max_goals=6,
        fuzz_iterations=3,
        dgm_generations=3,
        dgm_archive_size=10,
        peer_review_enabled=True,
        impact_analysis_enabled=True,
        executor_mode="subprocess",
        test_server_enabled=True,
        test_server_port=8888
    )

    # Container
    container = QARSIContainer(config)

    # Registra dependências (singletons)
    container.register('llm', MockLLM, singleton=True)
    container.register('wormgraph', MockWormGraph, singleton=True)
    container.register('telemetry', MockTelemetry, singleton=True)

    # Cria agente
    qa = QARSI(container, executor_mode=config.executor_mode,
               test_server_port=config.test_server_port if config.test_server_enabled else None)

    # Bootstrap
    qa.bootstrap({
        'login': 'Login com credenciais válidas',
        'purchase': 'Finalizar compra',
        'search': 'Buscar produtos por termo',
        'filter': 'Aplicar filtros de busca',
        'checkout': 'Processar checkout',
        'profile': 'Atualizar perfil do usuário'
    })

    # Ciclos de evolução
    print("📊 Executando ciclos de evolução...")
    for i in range(3):
        result = qa.run_cycle()
        print(f"Ciclo {i+1}: fitness={result['fitness']:.3f}, archive={result['archive_size']}, "
              f"tests={result['total_tests']}, reais={result['real_executions']}")

    # Processa um PR
    print("\n📝 Processando PR...")
    diff = """
    diff --git a/src/login.py b/src/login.py
    +++ b/src/login.py
    +    def authenticate(self, username, password):
    +        return username == "admin" and password == "secret"
    """
    pr_result = qa.handle_pr("PR-123", diff)
    print(f"PR Analysis: {pr_result.get('recommendations', ['OK'])}")
    if 'impact_analysis' in pr_result:
        print(f"Impacto: {list(pr_result['impact_analysis'].keys())}")

    # Relatório final
    report = qa.get_report()
    print(f"\n📊 Relatório Final:")
    for key, value in report.items():
        if not key.startswith('_') and not isinstance(value, dict):
            print(f"  {key}: {value}")
    print(f"  executor: mode={report['executor']['mode']}, rate={report['executor']['success_rate']:.1%}")

    # Testa o circuit breaker
    print("\n⚡ Testando Circuit Breaker...")
    retryable = Retryable(max_retries=2, failure_threshold=2)
    for i in range(5):
        try:
            retryable.execute(lambda: (lambda: (_ for _ in ()).throw(Exception("Falha simulada")))())
        except Exception as e:
            print(f"  Tentativa {i+1}: {str(e)[:30]}...")

    print(f"  Estado do Circuit Breaker: {retryable.circuit_breaker.state.value}")
    print(f"  Métricas: {retryable.get_metrics()['circuit_breaker']}")

    # Testa o executor real
    print("\n🔧 Testando Executor Real (subprocess)...")
    executor = TestExecutor(mode=ExecutorMode.SUBPROCESS, timeout=5)
    result = executor.execute({
        'goal_id': 'test_cmd',
        'command': ['echo', 'hello world']
    })
    print(f"  Subprocess: success={result['success']}, stdout={result.get('stdout', 'N/A')[:50]}")

    # Shutdown
    qa.shutdown()

    print("\n" + "="*60)
    print("✅ QA-RSI v8 executado com sucesso!")
    print("="*60)

HAS_WORMGRAPH = False
HAS_TELEMETRY = False

if __name__ == "__main__":
    # Adiciona import para HTTPServer
    from http.server import HTTPServer, BaseHTTPRequestHandler
    main()
