


import Mathlib.Data.Real.Basic
import Mathlib.Order.Basic
import Mathlib.Tactic.Linarith

namespace CathedralArkhe.Applications

variable (min max x : ℝ)

noncomputable def clamp (min max x : ℝ) : ℝ :=
  if x < min then min
  else if x > max then max
  else x

theorem clamp_within_bounds (h_min : min ≤ max) :
  min ≤ clamp min max x ∧ clamp min max x ≤ max := by
  dsimp [clamp]
  split_ifs with h1 h2
  · exact ⟨le_refl _, h_min⟩
  · exact ⟨h_min, le_refl _⟩
  · push Not at h1 h2
    exact ⟨h1, h2⟩

end CathedralArkhe.Applications
