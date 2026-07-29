"""
╔═══════════════════════════════════════════════════════════════════════════╗
║ CATHEDRAL ARKHE v16.0.0 — SUBSTRATO 3000 (Vision Transformer Core)      ║
║ Extrator de características visuais (ViT) para ambientes embodied.       ║
║ Selo: CATHEDRAL-ARKHE-v16.0.0-VISION-2026-06-14                         ║
║ Arquiteto: ORCID 0009-0005-2697-4668                                     ║
╚═══════════════════════════════════════════════════════════════════════════╝
"""

import torch
import torch.nn as nn
from typing import Tuple, Optional

try:
    import timm
    HAS_TIMM = True
except ImportError:
    HAS_TIMM = False

class VisionEncoder(nn.Module):
    """
    Encapsula um ViT pré-treinado (timm) e projeta a saída para o
    espaço latente padrão do Substrato 3000 (ex: 256D).
    """
    def __init__(self, model_name: str = "vit_tiny_patch16_224", embed_dim: int = 256,
                 freeze_backbone: bool = True, device: str = "cpu"):
        super().__init__()
        self.embed_dim = embed_dim
        self.device = torch.device(device)
        self._has_timm = HAS_TIMM

        if self._has_timm:
            # Carrega backbone ViT pré-treinado
            self.vit = timm.create_model(model_name, pretrained=True, num_classes=0)
            if freeze_backbone:
                for param in self.vit.parameters():
                    param.requires_grad = False
            vit_out_dim = self.vit.embed_dim
        else:
            # Fallback: CNN simples quando timm não disponível
            self.vit = _FallbackCNN(vit_out_dim := 192)
            self._has_timm = False

        # Projetor MLP para mapear para o espaço latente do Substrato
        self.projector = nn.Sequential(
            nn.Linear(vit_out_dim, embed_dim),
            nn.GELU(),
            nn.Linear(embed_dim, embed_dim)
        )

        self.layer_norm = nn.LayerNorm(embed_dim)
        self.to(self.device)

    def forward(self, x: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor]:
        """
        Args:
            x: Tensor de imagens [B, C, H, W] (normalizado segundo ImageNet)
        Returns:
            cls_token: Embedding denso [B, embed_dim] normalizado L2
            patch_tokens: Mapa de características [B, num_patches, embed_dim]
        """
        x = x.to(self.device)

        if self._has_timm:
            features = self.vit.forward_features(x)
            cls_token = features[:, 0, :]           # [B, vit_out_dim]
            patch_tokens = features[:, 1:, :]       # [B, num_patches, vit_out_dim]
        else:
            cls_token, patch_tokens = self.vit(x)

        cls_proj = self.projector(cls_token)        # [B, embed_dim]
        patch_proj = self.projector(patch_tokens)   # [B, num_patches, embed_dim]

        cls_proj = torch.nn.functional.normalize(cls_proj, p=2, dim=1)
        cls_proj = self.layer_norm(cls_proj)

        return cls_proj, patch_proj

    @torch.no_grad()
    def extract_for_cognition(self, observation: torch.Tensor) -> torch.Tensor:
        """Método otimizado para inferência pura (sem gradiente)."""
        self.eval()
        return self.forward(observation)



class _FallbackCNN(nn.Module):
    """CNN leve para fallback quando timm não está disponível."""
    def __init__(self, out_dim: int = 192):
        super().__init__()
        self.conv = nn.Sequential(
            nn.Conv2d(3, 32, 3, stride=2, padding=1), nn.ReLU(),
            nn.Conv2d(32, 64, 3, stride=2, padding=1), nn.ReLU(),
            nn.Conv2d(64, 128, 3, stride=2, padding=1), nn.ReLU(),
            nn.Conv2d(128, out_dim, 3, stride=2, padding=1), nn.ReLU(),
        )
        self.pool = nn.AdaptiveAvgPool2d(1)
        self.out_dim = out_dim
        self.embed_dim = out_dim

    def forward(self, x):
        feat = self.conv(x)                    # [B, out_dim, H', W']
        cls = self.pool(feat).squeeze(-1).squeeze(-1)  # [B, out_dim]
        B, C, H, W = feat.shape
        patches = feat.view(B, C, H * W).permute(0, 2, 1)  # [B, H*W, out_dim]
        return cls, patches
