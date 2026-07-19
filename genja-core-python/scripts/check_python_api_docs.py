"""Check documented Python API stubs and selected Rust/PyO3 bindings.

This script enforces documentation coverage for Python API surfaces that have
already been brought up to the repository's stub/docstring standard. It is
intentionally scoped: add files/classes to the constants below as more stubs and
Rust/PyO3 binding modules are audited.

The checks currently cover:

- public class and method docstrings in selected `.pyi` files;
- structural parity for Rust-backed classes duplicated between `genja.pyi` and
  the top-level `__init__.pyi` re-export surface;
- Rust doc comments on selected documented PyO3 bindings.

Run locally with:

    pdm run check-stubs
"""

from __future__ import annotations

import ast
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "python" / "genja"

STUBS_REQUIRING_DOCSTRINGS = [
    PACKAGE / "__init__.pyi",
    PACKAGE / "genja.pyi",
    PACKAGE / "plugin_manager.pyi",
    PACKAGE / "settings.pyi",
]

DUPLICATED_TOP_LEVEL_CLASSES = [
    "HostTaskResult",
    "TaskConnectionResolver",
    "TaskDefinition",
    "Tasks",
    "TaskResults",
    "Genja",
    "GenjaBuilder",
]

SETTINGS_RUST = ROOT / "src" / "settings.rs"
PLUGIN_MANAGER_RUST = ROOT / "src" / "plugin_manager.rs"

RUST_PYO3_DOC_CHECKS = [
    (SETTINGS_RUST, None),
    (PLUGIN_MANAGER_RUST, "impl PyPluginManager"),
]


def public_stub_members(tree: ast.Module) -> dict[str, ast.ClassDef]:
    return {
        node.name: node
        for node in tree.body
        if isinstance(node, ast.ClassDef) and not node.name.startswith("_")
    }


def class_api_shape(node: ast.ClassDef) -> set[tuple[str, str]]:
    shape: set[tuple[str, str]] = set()
    for child in node.body:
        if not isinstance(child, ast.FunctionDef):
            continue
        if child.name.startswith("_") and child.name not in {
            "__init__",
            "__getitem__",
            "__len__",
        }:
            continue
        kind = "method"
        for decorator in child.decorator_list:
            if isinstance(decorator, ast.Name) and decorator.id == "property":
                kind = "property"
                break
        shape.add((kind, child.name))
    return shape


def check_stub_docstrings(path: Path) -> list[str]:
    errors: list[str] = []
    tree = ast.parse(path.read_text(), filename=str(path))

    if ast.get_docstring(tree) is None and path.name != "genja.pyi":
        errors.append(f"{path}: missing module docstring")

    for class_node in public_stub_members(tree).values():
        if ast.get_docstring(class_node) is None:
            errors.append(f"{path}:{class_node.lineno}: class {class_node.name} missing docstring")

        for child in class_node.body:
            if not isinstance(child, ast.FunctionDef):
                continue
            if child.name.startswith("_") and child.name not in {
                "__init__",
                "__getitem__",
                "__len__",
            }:
                continue
            if ast.get_docstring(child) is None:
                errors.append(
                    f"{path}:{child.lineno}: {class_node.name}.{child.name} missing docstring"
                )

    return errors


def check_top_level_stub_parity() -> list[str]:
    errors: list[str] = []
    init_tree = ast.parse((PACKAGE / "__init__.pyi").read_text())
    genja_tree = ast.parse((PACKAGE / "genja.pyi").read_text())
    init_classes = public_stub_members(init_tree)
    genja_classes = public_stub_members(genja_tree)

    for class_name in DUPLICATED_TOP_LEVEL_CLASSES:
        if class_name not in init_classes:
            errors.append(f"__init__.pyi missing duplicated class {class_name}")
            continue
        if class_name not in genja_classes:
            errors.append(f"genja.pyi missing duplicated class {class_name}")
            continue

        init_shape = class_api_shape(init_classes[class_name])
        genja_shape = class_api_shape(genja_classes[class_name])
        if init_shape != genja_shape:
            missing = sorted(genja_shape - init_shape)
            extra = sorted(init_shape - genja_shape)
            if missing:
                errors.append(f"__init__.pyi {class_name} missing members: {missing}")
            if extra:
                errors.append(f"__init__.pyi {class_name} has extra members: {extra}")

    return errors


def previous_non_attribute_line(lines: list[str], index: int) -> str | None:
    cursor = index - 1
    while cursor >= 0:
        stripped = lines[cursor].strip()
        if not stripped or stripped.startswith("#["):
            cursor -= 1
            continue
        return stripped
    return None


def line_is_pyo3_method(stripped: str) -> bool:
    return stripped.startswith("fn ") or stripped.startswith("pub(crate) fn ")


def check_rust_pyo3_docs(path: Path, impl_scope: str | None) -> list[str]:
    errors: list[str] = []
    lines = path.read_text().splitlines()
    in_scope = impl_scope is None
    scope_depth: int | None = None

    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped == "#[cfg(test)]":
            break

        if impl_scope is not None and stripped.startswith(impl_scope):
            in_scope = True
            scope_depth = line.count("{") - line.count("}")
            continue

        if impl_scope is not None and in_scope and scope_depth is not None:
            scope_depth += line.count("{") - line.count("}")
            if scope_depth <= 0:
                in_scope = False
                scope_depth = None
                continue

        if stripped.startswith("#[pyclass("):
            previous = previous_non_attribute_line(lines, index)
            if previous is None or not previous.startswith("///"):
                errors.append(f"{path}:{index + 1}: pyclass missing Rust doc comment")
            continue

        if not in_scope:
            continue

        if stripped.startswith("fn __repr__") or stripped.startswith("pub fn register"):
            continue

        if line_is_pyo3_method(stripped):
            previous = previous_non_attribute_line(lines, index)
            if previous is None or not previous.startswith("///"):
                errors.append(f"{path}:{index + 1}: PyO3 method missing Rust doc comment")

    return errors


def main() -> int:
    errors: list[str] = []
    for path in STUBS_REQUIRING_DOCSTRINGS:
        errors.extend(check_stub_docstrings(path))
    errors.extend(check_top_level_stub_parity())
    for path, impl_scope in RUST_PYO3_DOC_CHECKS:
        errors.extend(check_rust_pyo3_docs(path, impl_scope))

    if errors:
        print("Python API documentation checks failed:")
        for error in errors:
            print(f"- {error}")
        return 1

    print("Python API documentation checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
