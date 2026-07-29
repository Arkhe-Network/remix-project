#!/usr/bin/env python3
"""
Cathedral ARKHE v17.2 – zkHydra Gateway
Orquestra análise de segurança de circuitos ZK (Circom) usando o framework zkHydra.
"""

import asyncio
import json
import logging
import subprocess
import argparse
import sys
from pathlib import Path
from typing import List, Dict, Optional

logger = logging.getLogger("cathedral.zkhydra")
logging.basicConfig(level=logging.INFO, format="%(levelname)s:%(name)s:%(message)s")

class ZkHydraResult:
    """Estrutura para resultados de análise do zkHydra."""
    def __init__(self, tool_name: str, raw_output: dict):
        self.tool_name = tool_name
        self.has_findings = raw_output.get("findings", []) != []
        self.findings = raw_output.get("findings", [])

class ZkHydraGateway:
    """
    Gateway para o framework zkHydra.
    Executa análises de segurança sobre circuitos Circom e retorna resultados estruturados.
    """

    def __init__(self, work_dir: str = "./zkhydra_work"):
        self.work_dir = Path(work_dir)
        self.work_dir.mkdir(parents=True, exist_ok=True)

    async def analyze_circuit(
        self,
        circuit_path: str,
        tools: List[str] = None,
        timeout: int = 600
    ) -> Dict[str, ZkHydraResult]:
        """
        Executa o zkHydra (modo analyze) sobre um arquivo .circom.

        Args:
            circuit_path: Caminho para o arquivo .circom.
            tools: Lista de ferramentas a usar (ex: ["circomspect", "circom_civer"]).
            timeout: Tempo limite em segundos.

        Returns:
            Dicionário mapeando nome da ferramenta para ZkHydraResult.
        """
        if tools is None or "all" in tools:
            tools = ["circomspect", "circom_civer", "picus"]

        output_dir = self.work_dir / "output"
        output_dir.mkdir(exist_ok=True)

        # Monta o comando conforme documentação do zkHydra
        # Exemplo: uv run python -m zkhydra.main analyze --input circuit.circom --tools circomspect,circom_civer --output results/
        # Na prática, usaremos o container Docker para garantir reprodutibilidade.
        cmd = [
            "docker", "run", "--rm",
            "-v", f"{Path(circuit_path).resolve().parent}:/zkhydra/input",
            "-v", f"{output_dir.resolve()}:/zkhydra/output",
            "ghcr.io/zksecurity/zkhydra:latest",
            "uv", "run", "python", "-m", "zkhydra.main", "analyze",
            "--input", f"/zkhydra/input/{Path(circuit_path).name}",
            "--tools", ",".join(tools),
            "--timeout", str(timeout),
            "--output", "/zkhydra/output"
        ]

        try:
            logger.info(f"Executando comando zkHydra...")
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode != 0:
                raise RuntimeError(f"zkHydra falhou (RC={proc.returncode}): {stderr.decode()}")

            # O zkHydra gera um arquivo results.json por ferramenta dentro de output_dir
            results = {}
            for tool in tools:
                result_file = output_dir / tool / "results.json"
                if result_file.exists():
                    with open(result_file, "r") as f:
                        raw = json.load(f)
                        results[tool] = ZkHydraResult(tool, raw)
                else:
                    logger.warning(f"Resultado da ferramenta {tool} não encontrado em {result_file}")
            return results

        except Exception as e:
            logger.error(f"Erro ao executar zkHydra: {e}")
            raise  # Let it fail so the CI can fail if something breaks instead of silently succeeding

    async def evaluate_vulnerability(self, bug_config_path: str, tools: List[str] = None) -> Dict:
        """
        Executa o modo 'evaluate' do zkHydra, comparando resultados com vulnerabilidades conhecidas (zkbugs).
        Útil para benchmarking e validação de novas ferramentas.
        """
        if tools is None:
            tools = ["circomspect", "circom_civer", "picus", "zkfuzz"]

        output_dir = self.work_dir / "evaluate_output"
        output_dir.mkdir(exist_ok=True)

        cmd = [
            "docker", "run", "--rm",
            "-v", f"{Path(bug_config_path).resolve().parent}:/zkhydra/bug",
            "-v", f"{output_dir.resolve()}:/zkhydra/output",
            "ghcr.io/zksecurity/zkhydra:latest",
            "uv", "run", "python", "-m", "zkhydra.main", "evaluate",
            "--input", f"/zkhydra/bug/{Path(bug_config_path).name}",
            "--tools", ",".join(tools),
            "--output", "/zkhydra/output"
        ]

        try:
            proc = await asyncio.create_subprocess_exec(*cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
            stdout, stderr = await proc.communicate()
            if proc.returncode != 0:
                raise RuntimeError(f"zkHydra evaluate falhou: {stderr.decode()}")

            # Processa o arquivo evaluation.json gerado
            eval_file = output_dir / "evaluation.json"
            if eval_file.exists():
                with open(eval_file, "r") as f:
                    return json.load(f)
            else:
                logger.warning("Arquivo evaluation.json não encontrado")
                return {}
        except Exception as e:
            logger.error(f"Erro no evaluate: {e}")
            return {}

async def process_zkhydra_results(results: Dict[str, ZkHydraResult]) -> None:
    for tool, res in results.items():
        if res.has_findings:
            logger.warning(f"[{tool}] Vulnerabilidades detectadas: {res.findings}")
        else:
            logger.info(f"[{tool}] Nenhuma vulnerabilidade encontrada.")

async def main():
    parser = argparse.ArgumentParser(description="Cathedral ARKHE v17.2 - zkHydra Gateway")
    subparsers = parser.add_subparsers(dest="command", required=True)

    analyze_parser = subparsers.add_parser("analyze")
    analyze_parser.add_argument("--circuit", required=True, help="Path to the .circom file")
    analyze_parser.add_argument("--tools", default="all", help="Comma-separated tools or 'all'")
    analyze_parser.add_argument("--fail-on-finding", action="store_true", help="Fail if findings exist")
    analyze_parser.add_argument("--output", help="Output JSON file for the combined report")

    args = parser.parse_args()

    if args.command == "analyze":
        gateway = ZkHydraGateway()
        tools_list = None if args.tools == "all" else args.tools.split(",")
        try:
            results = await gateway.analyze_circuit(args.circuit, tools=tools_list)
        except Exception:
            sys.exit(1)

        await process_zkhydra_results(results)

        if args.output:
            report = {tool: {"has_findings": res.has_findings, "findings": res.findings} for tool, res in results.items()}
            with open(args.output, "w") as f:
                json.dump(report, f, indent=2)

        if args.fail_on_finding:
            for tool, res in results.items():
                if res.has_findings:
                    logger.error("Vulnerabilidades encontradas. Falhando a execução (--fail-on-finding ativo).")
                    sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
