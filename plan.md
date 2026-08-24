1. **Create .github/workflows/security-audit.yml**
   - Content from the prompt.
2. **Create .github/workflows/score.yml**
   - Content from the prompt.
3. **Create scripts/score-crates.py**
   - Content from the prompt. Ensure the script is executable.
4. **Create deny.toml in the workspace root**
   - Use the specific policies requested (advisories CVSS>=4.0 deny, licenses GPL warn, bans openssl/failure/rustc-serialize, sources only crates.io).
5. **Install cargo-deny and run `cargo deny check`**
   - Validate the configuration and check for any license/vulnerability violations that need fixing.
6. **Install cargo-cyclonedx and run `cargo cyclonedx --format json --output sbom.json`**
   - Check if SBOM generation works.
7. **Run `python3 scripts/score-crates.py --json --badge`**
   - Validate that scoring is working and check the global score.
8. **Fix any license/vulnerability violations and errors as requested by the user**
   - If cargo-deny finds any issues, fix them before the merge.
9. **Complete pre-commit steps**
   - Follow `pre_commit_instructions` to ensure proper testing, verifications, reviews, and reflections are done.
10. **Submit the changes**
   - Submit via the submit tool with a descriptive commit message.
