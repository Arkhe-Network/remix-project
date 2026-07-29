import numpy as np
from dataclasses import dataclass
from typing import Dict, List

@dataclass
class EmergenceProfile:
    domain: str
    geometry: str
    peak_layer: int
    stability: str

class PerceptualGeometryEngine:
    """
    Substrato 934 — PERCEPTUAL-GEOMETRY-EMERGENCE
    Analyzes the emergence of perceptual geometries in LLM representations.
    Based on arXiv:2605.27970v1.
    """
    def __init__(self):
        self.profiles = {
            "color": EmergenceProfile("color", "Circular manifold (color wheel)", 12, "Transient, attenuates in late layers"),
            "emotion": EmergenceProfile("emotion", "Valence-arousal structure (circumplex)", 16, "Persistent through late layers"),
            "pitch": EmergenceProfile("pitch", "Arc-like continuous ordinal manifold", 14, "Progressive deformation in late layers"),
            "taste": EmergenceProfile("taste", "Organized taste manifold", 8, "Rapid degradation, noisy trajectory"),
        }

    def analyze_layer(self, domain: str, layer_idx: int) -> Dict:
        """
        Simulates organization score (RSA/GPA) based on layer depth.
        Trajectory: weak → organized → attenuated across depth.
        """
        if domain not in self.profiles:
            return {"error": f"Domain {domain} not found"}

        profile = self.profiles[domain]
        # Simulate organization score based on peak layer (Gaussian-like)
        distance = abs(layer_idx - profile.peak_layer)
        score = np.exp(-(distance**2) / 32.0)

        return {
            "domain": domain,
            "layer_idx": layer_idx,
            "organization_score": round(float(score), 4),
            "geometry": profile.geometry,
            "stability_status": profile.stability if layer_idx >= profile.peak_layer else "Organizing"
        }

    def get_manifold_coords(self, domain: str, num_points: int = 10) -> List[Dict]:
        """Simulates MDS coordinates for the domain manifold."""
        coords = []
        for i in range(num_points):
            angle = 2 * np.pi * i / num_points
            if domain == "color":
                coords.append({"id": i, "x": np.cos(angle), "y": np.sin(angle)})
            elif domain == "pitch":
                # Arc-like manifold
                coords.append({"id": i, "x": i / num_points, "y": np.sin(np.pi * i / num_points)})
            elif domain == "emotion":
                # Circumplex (valence-arousal)
                coords.append({"id": i, "valence": np.cos(angle), "arousal": np.sin(angle)})
            else:
                # Noisy trajectory
                coords.append({"id": i, "x": np.random.randn(), "y": np.random.randn()})
        return coords

    def rsa_alignment(self, model_rdm: np.ndarray, human_rdm: np.ndarray) -> float:
        """Simulates Spearman rank correlation (RSA)."""
        return float(np.corrcoef(model_rdm.flatten(), human_rdm.flatten())[0, 1])
