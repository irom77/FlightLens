#!/usr/bin/env python3
import subprocess
import sys
from pathlib import Path

MAX_ROUNDS = 4
PROMPTS_DIR = Path(".codex/prompts")
TEST_COMMAND = ["pnpm", "test"]

def run_cmd(cmd: list[str]) -> tuple[int, str, str]:
    res = subprocess.run(cmd, capture_output=True, text=True)
    return res.returncode, res.stdout, res.stderr

def run_codex(instruction: str, system_prompt_file: Path) -> str:
    """Invokes Codex CLI non-interactively with a specific persona/prompt."""
    cmd = [
        "codex", "exec",  # Use your CLI's non-interactive/exec flag
        "--system-prompt-file", str(system_prompt_file),
        instruction
    ]
    code, out, err = run_cmd(cmd)
    if code != 0:
        print(f"[Error running codex]: {err}", file=sys.stderr)
    return out

def get_git_diff() -> str:
    _, diff, _ = run_cmd(["git", "diff", "HEAD"])
    return diff

def main():
    if len(sys.argv) < 2:
        print("Usage: python .codex/scripts/adversarial_review.py \"Task description\"")
        sys.exit(1)

    task_spec = sys.argv[1]
    coder_prompt = PROMPTS_DIR / "coder.md"
    reviewer_prompt = PROMPTS_DIR / "reviewer.md"

    current_instruction = f"Task: {task_spec}\nImplement the solution and write tests."

    for round_idx in range(1, MAX_ROUNDS + 1):
        print(f"\n==================== ROUND {round_idx} ====================")
        print("[1/3] Coder applying changes...")
        run_codex(current_instruction, coder_prompt)

        # 1. Run local test sandbox / test runner
        print("[2/3] Running tests...")
        test_exit, test_stdout, test_stderr = run_cmd(TEST_COMMAND)
        test_output = f"{test_stdout}\n{test_stderr}".strip()
        tests_passed = (test_exit == 0)

        # 2. Get the Git diff of what Coder actually changed
        diff = get_git_diff()
        if not diff:
            print("[!] No git diff detected. Coder produced no modifications.")

        # 3. Adversarial Reviewer checks diff + test run
        print("[3/3] Reviewer assessing diff and test outputs...")
        reviewer_payload = f"""Task: {task_spec}

Git Diff:
```diff
{diff}
```

Test Results (Success: {tests_passed}):
```
{test_output}
```
"""
        reviewer_output = run_codex(reviewer_payload, reviewer_prompt)
        print("\n--- Reviewer Feedback ---")
        print(reviewer_output)
        print("-------------------------\n")

        if "STATUS: APPROVED" in reviewer_output:
            print(f"🎉 Success! Adversarial review passed on Round {round_idx}!")
            sys.exit(0)
        else:
            print(f"❌ Round {round_idx} Rejected. Preparing feedback for Round {round_idx + 1}...")
            current_instruction = f"""Task: {task_spec}

Your previous implementation was REJECTED by the reviewer.
Please address the following feedback and update the code/tests.

Reviewer Feedback:
{reviewer_output}
"""

    print("⚠️ Reached maximum rounds without approval.")
    sys.exit(1)

if __name__ == "__main__":
    main()
