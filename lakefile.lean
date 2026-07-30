import Lake
open Lake DSL

package «CathedralArkhe» where
  -- add package configuration options here

lean_lib «CathedralArkhe» where
  roots := #[
    `CathedralArkhe.T1.Mobius,
    `CathedralArkhe.Abstract.QuotientTower,
    `CathedralArkhe.Abstract.AgentCore,
    `CathedralArkhe.Applications.MathInCSSJS,
    `CathedralArkhe.Applications.ResponsiveLayout,
    `CathedralArkhe.Applications.Typography
  ]

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git"
