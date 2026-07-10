"""Drift guard: every public name exposed by highuvlith._native must
appear in python/highuvlith/_native.pyi, so the type stub cannot silently
fall behind the PyO3 bindings again."""

import ast
import inspect
import pathlib

import highuvlith
import highuvlith._native as native


def _parse_stub():
    stub_path = pathlib.Path(highuvlith.__file__).parent / "_native.pyi"
    tree = ast.parse(stub_path.read_text())

    stub_classes: dict[str, set[str]] = {}
    stub_functions: set[str] = set()

    for node in tree.body:
        if isinstance(node, ast.ClassDef):
            members: set[str] = set()
            for sub in node.body:
                if isinstance(sub, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    members.add(sub.name)
                elif isinstance(sub, ast.AnnAssign) and isinstance(
                    sub.target, ast.Name
                ):
                    members.add(sub.target.id)
            stub_classes[node.name] = members
        elif isinstance(node, ast.FunctionDef):
            stub_functions.add(node.name)

    return stub_classes, stub_functions


def test_native_classes_and_functions_in_stub():
    stub_classes, stub_functions = _parse_stub()

    for name in dir(native):
        if name.startswith("_"):
            continue
        obj = getattr(native, name)
        if inspect.isclass(obj):
            assert name in stub_classes, f"class {name} missing from _native.pyi"
        else:
            assert name in stub_functions, (
                f"function {name} missing from _native.pyi"
            )


def test_native_class_members_in_stub():
    stub_classes, _ = _parse_stub()

    for name in dir(native):
        if name.startswith("_"):
            continue
        obj = getattr(native, name)
        if not inspect.isclass(obj):
            continue
        stub_members = stub_classes.get(name, set())
        for attr in dir(obj):
            if attr.startswith("_"):
                continue
            assert attr in stub_members, (
                f"{name}.{attr} exposed by the native module but missing "
                f"from _native.pyi — update the stub in the same PR"
            )
