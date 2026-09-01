module {
  func.func @simulate_mhd_3d(%ic: tensor<3x3x3x8xf64>, %t0: f64, %t1: f64) -> tensor<3x3x3x8xf64> {
    %state = arkhe.plasma.init %ic : tensor<3x3x3x8xf64>
    %result = scf.for %i = 0 to 100 step 1 {
      %state = arkhe.plasma.step %state, %t0, %t1 : tensor<...>
    } : tensor<...>
    return %result
  }
}