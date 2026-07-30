
import Mathlib.Data.Real.Basic

namespace CathedralArkhe.AgentCore

universe u v

abbrev Action (A : Type u) := A

abbrev Observation (O : Type v) := O

abbrev State (S : Type u) := S

class Distribution (α : Type u) where
  support : α → ℝ
  nonneg : ∀ a, 0 ≤ support a
  totalMass : ℝ

abbrev Belief (S : Type u) [Distribution S] := Distribution S

structure Policy (S A : Type u) [Distribution S] where
  eval : S → A → ℝ
  nonneg : ∀ s a, 0 ≤ eval s a
  norm : ∀ _s : S, (Distribution.totalMass (α := S)) = 1

structure WorldModel (S A : Type u) [Distribution S] where
  trans : S → A → S → ℝ
  nonneg : ∀ s a s', 0 ≤ trans s a s'
  norm : ∀ (_s : S) (_a : A), (Distribution.totalMass (α := S)) = 1

structure Experience (S A : Type u) where
  stateBefore : S
  action : A
  stateAfter : S

structure AgentState (S A : Type u) [Distribution S] where
  belief : Belief S
  policy : Policy S A
  worldModel : WorldModel S A

structure Agent (S A : Type u) [Distribution S] where
  state : AgentState S A
  update : AgentState S A → Experience S A → AgentState S A

def IsExperienceLearner (S A : Type u) [Distribution S] (agent : Agent S A) : Prop :=
  ∃ (exp : Experience S A), agent.update agent.state exp ≠ agent.state

end CathedralArkhe.AgentCore
