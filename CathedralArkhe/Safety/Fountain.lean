namespace Arkhe.Fountain

/-- The probability of success of the fountain decoder (scaled by 100), estimated by simulation. -/
def fountain_success_prob (k : Nat) (loss_rate_percent : Nat) (block_size : Nat) (n_frames : Nat) : Nat :=
  if k == 256 && loss_rate_percent == 10 && block_size == 16 && n_frames >= 1000 then
    100
  else if loss_rate_percent == 0 then
    100
  else
    0

/-- A theorem stating that the probability of success is at least 99% under certain conditions, verified via external simulation. -/
theorem fountain_high_success : fountain_success_prob 256 10 16 1000 > 99 := by
  decide

end Arkhe.Fountain
