@echo off
echo === Running Soroban Formal Invariants (Windows) ===
cargo test --lib test_invariant
if %ERRORLEVEL% equ 0 (
  echo Invariant verification passed.
) else (
  echo Invariant verification failed.
  exit /b %ERRORLEVEL%
)
