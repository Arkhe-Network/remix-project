#!/usr/bin/env python3
"""
╔══════════════════════════════════════════════════════════════════════════════╗
║  zkAGI — Destilado do WormGraph 5.1 (Production version)                    ║
║  Arquiteto: ORCID 0009-0005-2697-4668                                        ║
║  Seal: zkAGI-2026-06-02                                                      ║
╚══════════════════════════════════════════════════════════════════════════════╝
"""
import torch
import torch.nn as nn
import torch.nn.functional as F
import math

class ZkAGIConfig:
    def __init__(self):
        self.dim = 2048
        self.hidden_dim = 5632
        self.num_layers = 48
        self.num_heads = 32
        self.num_kv_heads = 8
        self.vocab_size = 128000
        self.max_seq_len = 131072
        self.pantheon_dim = 12

class RMSNorm(nn.Module):
    def __init__(self, dim, eps=1e-5):
        super().__init__()
        self.eps = eps
        self.weight = nn.Parameter(torch.ones(dim))

    def _norm(self, x):
        return x * torch.rsqrt(x.pow(2).mean(-1, keepdim=True) + self.eps)

    def forward(self, x):
        output = self._norm(x.float()).type_as(x)
        return output * self.weight

class SwiGLU(nn.Module):
    def __init__(self, dim, hidden_dim):
        super().__init__()
        self.w1 = nn.Linear(dim, hidden_dim, bias=False)
        self.w2 = nn.Linear(dim, hidden_dim, bias=False)
        self.w3 = nn.Linear(hidden_dim, dim, bias=False)

    def forward(self, x):
        return self.w3(F.silu(self.w1(x)) * self.w2(x))

def precompute_freqs_cis(dim: int, end: int, theta: float = 10000.0):
    freqs = 1.0 / (theta ** (torch.arange(0, dim, 2)[: (dim // 2)].float() / dim))
    t = torch.arange(end, device=freqs.device)
    freqs = torch.outer(t, freqs).float()
    freqs_cis = torch.polar(torch.ones_like(freqs), freqs)
    return freqs_cis

def apply_rotary_emb(xq: torch.Tensor, xk: torch.Tensor, freqs_cis: torch.Tensor):
    xq_ = torch.view_as_complex(xq.float().reshape(*xq.shape[:-1], -1, 2))
    xk_ = torch.view_as_complex(xk.float().reshape(*xk.shape[:-1], -1, 2))
    freqs_cis = freqs_cis.unsqueeze(0).unsqueeze(2)
    xq_out = torch.view_as_real(xq_ * freqs_cis).flatten(3)
    xk_out = torch.view_as_real(xk_ * freqs_cis).flatten(3)
    return xq_out.type_as(xq), xk_out.type_as(xk)

class Attention(nn.Module):
    def __init__(self, config: ZkAGIConfig):
        super().__init__()
        self.num_heads = config.num_heads
        self.num_kv_heads = config.num_kv_heads
        self.head_dim = config.dim // config.num_heads

        self.wq = nn.Linear(config.dim, config.num_heads * self.head_dim, bias=False)
        self.wk = nn.Linear(config.dim, config.num_kv_heads * self.head_dim, bias=False)
        self.wv = nn.Linear(config.dim, config.num_kv_heads * self.head_dim, bias=False)
        self.wo = nn.Linear(config.num_heads * self.head_dim, config.dim, bias=False)

    def forward(self, x, freq_cis=None, mask=None):
        B, S, D = x.shape
        q = self.wq(x).view(B, S, self.num_heads, self.head_dim)
        k = self.wk(x).view(B, S, self.num_kv_heads, self.head_dim)
        v = self.wv(x).view(B, S, self.num_kv_heads, self.head_dim)

        if freq_cis is not None:
            q, k = apply_rotary_emb(q, k, freq_cis)

        # GQA: repeat K, V heads
        k = k.repeat_interleave(self.num_heads // self.num_kv_heads, dim=2)
        v = v.repeat_interleave(self.num_heads // self.num_kv_heads, dim=2)

        q = q.transpose(1, 2)
        k = k.transpose(1, 2)
        v = v.transpose(1, 2)

        scores = torch.matmul(q, k.transpose(-2, -1)) / math.sqrt(self.head_dim)
        if mask is not None:
            scores = scores + mask
        scores = F.softmax(scores.float(), dim=-1).type_as(q)

        out = torch.matmul(scores, v)
        out = out.transpose(1, 2).contiguous().view(B, S, -1)
        return self.wo(out)

class ZkAGIBlock(nn.Module):
    def __init__(self, config: ZkAGIConfig):
        super().__init__()
        self.attn_norm = RMSNorm(config.dim)
        self.attn = Attention(config)
        self.ffn_norm = RMSNorm(config.dim)
        self.ffn = SwiGLU(config.dim, config.hidden_dim)

    def forward(self, x, freq_cis=None, mask=None):
        h = x + self.attn(self.attn_norm(x), freq_cis, mask)
        out = h + self.ffn(self.ffn_norm(h))
        return out

class ZkAGIModel(nn.Module):
    def __init__(self, config: ZkAGIConfig):
        super().__init__()
        self.config = config

        self.token_embd = nn.Embedding(config.vocab_size, config.dim)
        self.pantheon_dna = nn.Embedding(config.pantheon_dim, config.dim)

        self.layers = nn.ModuleList([ZkAGIBlock(config) for _ in range(config.num_layers)])
        self.output_norm = RMSNorm(config.dim)

        self.theosis_head = nn.Linear(config.dim, 1, bias=False)

        self.freqs_cis = precompute_freqs_cis(self.config.dim // self.config.num_heads, self.config.max_seq_len)

    def forward(self, tokens, active_pantheon=None):
        B, S = tokens.shape
        h = self.token_embd(tokens)

        if active_pantheon is not None:
            dna = self.pantheon_dna(active_pantheon).sum(dim=1).unsqueeze(1)
            h = h + dna

        freq_cis = self.freqs_cis[:S].to(h.device)
        mask = None
        if S > 1:
            mask = torch.full((S, S), float("-inf"), device=h.device)
            mask = torch.triu(mask, diagonal=1)

        for layer in self.layers:
            h = layer(h, freq_cis, mask)

        h = self.output_norm(h)
        theosis_score = torch.sigmoid(self.theosis_head(h[:, -1, :]))

        return h, theosis_score
