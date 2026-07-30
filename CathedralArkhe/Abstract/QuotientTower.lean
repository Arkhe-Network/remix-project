import Mathlib.Algebra.Group.Action.Basic

namespace CathedralArkhe.Abstract

universe u v w

def orbitRel {G : Type u} [Group G] {α : Type v} [MulAction G α] (x y : α) : Prop :=
  ∃ g : G, g • x = y

theorem orbitRel_refl {G : Type u} [Group G] {α : Type v} [MulAction G α] (x : α) : orbitRel (G := G) x x :=
  ⟨1, one_smul G x⟩

theorem orbitRel_symm {G : Type u} [Group G] {α : Type v} [MulAction G α] {x y : α} : orbitRel (G := G) x y → orbitRel (G := G) y x := by
  rintro ⟨g, hg⟩
  exact ⟨g⁻¹, by rw [← hg, inv_smul_smul]⟩

theorem orbitRel_trans {G : Type u} [Group G] {α : Type v} [MulAction G α] {x y z : α} : orbitRel (G := G) x y → orbitRel (G := G) y z → orbitRel (G := G) x z := by
  rintro ⟨g1, hg1⟩ ⟨g2, hg2⟩
  exact ⟨g2 * g1, by rw [mul_smul, hg1, hg2]⟩

def orbitSetoid (G : Type u) [Group G] (α : Type v) [MulAction G α] : Setoid α where
  r := orbitRel (G := G)
  iseqv := ⟨orbitRel_refl (G := G), orbitRel_symm (G := G), orbitRel_trans (G := G)⟩

end CathedralArkhe.Abstract
