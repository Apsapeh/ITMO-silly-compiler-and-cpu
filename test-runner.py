import yaml
import subprocess
import difflib
import argparse
import re
from pathlib import Path

GOLDEN_DIR = Path("golden")
BUILD_DIR = Path(".golden-build")

# Temp files
SRC_FILE = BUILD_DIR / "src.shit"
ASM_FILE = BUILD_DIR / "asm.s"
BIN_OUT_FILE = BUILD_DIR / "out.bin"
COMPILER_LOG_FILE = BUILD_DIR / "compiler.log"
CPU_MEM_FILE = BUILD_DIR / "mem.log"
CPU_LOG_FILE = BUILD_DIR / "cpu.log"


def setup_build_dir():
    BUILD_DIR.mkdir(exist_ok=True)


def normalize_text(text: str) -> str:
    # Remove trash from the string
    if not text:
        return ""
    text = text.replace("\t", "    ").replace("\xa0", " ")
    lines = text.replace("\r\n", "\n").split("\n")
    return "\n".join(line.rstrip() for line in lines).strip()


def run_command(cmd, output_file=None):
    print(f"  [EXEC] {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)
    if output_file:
        output_file.write_text(result.stdout, encoding="utf-8")
    if result.returncode != 0:
        print(f"  [ERROR] Command exited with code {result.returncode}")
        print(result.stderr)
    return result.stdout, result.returncode


def update_yaml_field(
    yaml_text: str, key: str, value: str, is_block: bool = False
) -> str:
    # Update or add fields to the yaml-test
    if is_block:
        indented_value = "\n".join(f"  {line}" for line in value.splitlines())
        field_text = f"{key}: |\n{indented_value}"
    else:
        field_text = f"{key}: {value}"

    pattern = rf"^{key}:\s*.*?(?=(?:\n\S)|\Z)"
    if re.search(pattern, yaml_text, re.MULTILINE | re.DOTALL):
        yaml_text = re.sub(
            pattern, field_text, yaml_text, flags=re.MULTILINE | re.DOTALL
        )
    else:
        yaml_text = yaml_text.rstrip() + f"\n\n{field_text}\n"
    return yaml_text


def process_golden_file(yaml_path: Path, update_mode: bool):
    print(f"\n=== Running test: {yaml_path.name} ===")

    yaml_text = yaml_path.read_text(encoding="utf-8")
    golden = yaml.safe_load(yaml_text)

    # Prepare sorce codes
    SRC_FILE.write_text(golden.get("src", ""), encoding="utf-8")
    ASM_FILE.write_text(golden.get("asm", ""), encoding="utf-8")

    # Compiler
    compiler_cmd = [
        "cargo",
        "run",
        "--bin",
        "compiler",
        "--",
        "-s",
        str(SRC_FILE),
        "-a",
        str(ASM_FILE),
        "-o",
        str(BIN_OUT_FILE),
        "-d",
    ]

    compiler_out, cc_code = run_command(compiler_cmd, COMPILER_LOG_FILE)
    if cc_code != 0:
        return False

    actual_compiler_log = normalize_text(compiler_out)

    if not update_mode:
        # Check complier output if it exists
        expected_compiler_log = normalize_text(golden.get("compiler_log", ""))
        if expected_compiler_log and expected_compiler_log != actual_compiler_log:
            print("  [FAIL] The compiler output does not match the YAML reference!")
            diff = list(
                difflib.unified_diff(
                    expected_compiler_log.splitlines(),
                    actual_compiler_log.splitlines(),
                    fromfile="expected",
                    tofile="actual",
                    lineterm="",
                )
            )
            print("  [DIFF] Difference:")
            for line in diff[:30]:
                if line.startswith("+"):
                    print(f"    \033[92m{line}\033[0m")
                elif line.startswith("-"):
                    print(f"    \033[91m{line}\033[0m")
                else:
                    print(f"    {line}")
            return False
        elif expected_compiler_log:
            print("  [SUCCESS] The compile output match the YAML refernce")

    # CPU
    limit = str(golden.get("limit", 1000000))
    cpu_log_mode = str(golden.get("cpu_log_limit", "all")).strip().lower()

    cpu_cmd = [
        "cargo",
        "run",
        "--bin",
        "cpu",
        "--",
        "-b",
        str(BIN_OUT_FILE),
        "-l",
        limit,
        "-o",
        str(CPU_MEM_FILE),
    ]

    if cpu_log_mode != "none":
        cpu_cmd.append("-d")

    inputs = golden.get("input", {})
    if inputs:
        for addr, val in inputs.items():
            cpu_cmd.extend(["-i", f"{addr}:{val}"])

    cpu_out_log, cpu_code = run_command(cpu_cmd, CPU_LOG_FILE)
    if cpu_code != 0:
        return False

    # CPU result output
    actual_cpu_out = []
    if CPU_MEM_FILE.exists():
        content = CPU_MEM_FILE.read_text(encoding="utf-8").strip()
        if content:
            actual_cpu_out = [
                int(line.strip()) for line in content.split("\n") if line.strip()
            ]

    # CPU log
    actual_cpu_log = CPU_LOG_FILE.read_text(encoding="utf-8").strip()
    if cpu_log_mode != "all" and cpu_log_mode.isdigit():
        actual_cpu_log = "\n".join(
            actual_cpu_log.split("\n")[: int(cpu_log_mode)]
        ).strip()
    actual_cpu_log = normalize_text(actual_cpu_log)

    if update_mode:
        yaml_text = update_yaml_field(
            yaml_text, "cpu_out", str(actual_cpu_out), is_block=False
        )
        yaml_text = update_yaml_field(
            yaml_text, "compiler_log", actual_compiler_log, is_block=True
        )

        if cpu_log_mode != "none":
            yaml_text = update_yaml_field(
                yaml_text, "cpu_log", actual_cpu_log, is_block=True
            )
        else:
            yaml_text = re.sub(
                r"^cpu_log:\s*.*?(?=(?:\n\S)|\Z)",
                "",
                yaml_text,
                flags=re.MULTILINE | re.DOTALL,
            )

        yaml_path.write_text(yaml_text, encoding="utf-8")
        print(
            f"  [UPD] Logs (cpu_out, compiler_log, cpu_log) successfully updated in the {yaml_path.name}"
        )
        return True

    else:
        expected_cpu_out = golden.get("cpu_out", [])
        if actual_cpu_out == expected_cpu_out:
            print(f"  [SUCCESS] CPU output is match: {actual_cpu_out}")
        else:
            print(f"  [FAIL] Expected: {expected_cpu_out}, actual: {actual_cpu_out}")
            return False

        if cpu_log_mode != "none":
            expected_cpu_log = normalize_text(golden.get("cpu_log", ""))
            if expected_cpu_log and expected_cpu_log != actual_cpu_log:
                print("  [FAIL] CPU log doesn't match the YAML reference!")
                return False
            elif expected_cpu_log:
                print("  [SUCCESS] CPU log match the YAML reference")

    return True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--upd",
        action="store_true",
        help="Gen and update logs in YAML",
    )
    args = parser.parse_args()

    if not GOLDEN_DIR.exists():
        print(f"Dir '{GOLDEN_DIR}' not found!")
        return

    setup_build_dir()
    yaml_files = list(GOLDEN_DIR.glob("*.yaml"))

    passed = 0
    total = len(yaml_files)

    for yaml_path in yaml_files:
        # Skip template test
        if str(yaml_path).endswith("template.yaml"):
            total -= 1
            continue

        if process_golden_file(yaml_path, update_mode=args.upd):
            passed += 1

    if args.upd:
        print(
            f"\n=== Summary: References successfully updated in the YAML for {passed} from {total} tests ==="
        )
    else:
        print(f"\n=== Summary: Passed {passed} from {total} tests ===")
        if passed != total:
            exit(1)


if __name__ == "__main__":
    main()
