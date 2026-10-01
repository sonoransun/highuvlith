"""Drift guard: every public name exposed by highuvlith._native must
appear in python/highuvlith/_native.pyi with the same call signature
(parameter names, order, kind, and default values) and the same member kind
(property / staticmethod / method), so the type stub cannot silently fall
behind the PyO3 bindings again."""

import ast
import inspect
import pathlib

import highuvlith
import highuvlith._native as native

_MISSING = object()


def _parse_stub():
    stub_path = pathlib.Path(highuvlith.__file__).parent / "_native.pyi"
    tree = ast.parse(stub_path.read_text())

    stub_classes: dict[str, dict[str, ast.AST]] = {}
    stub_functions: dict[str, ast.FunctionDef] = {}

    for node in tree.body:
        if isinstance(node, ast.ClassDef):
            members: dict[str, ast.AST] = {}
            for sub in node.body:
                if isinstance(sub, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    members[sub.name] = sub
                elif isinstance(sub, ast.AnnAssign) and isinstance(sub.target, ast.Name):
                    members[sub.target.id] = sub
            stub_classes[node.name] = members
        elif isinstance(node, ast.FunctionDef):
            stub_functions[node.name] = node

    return stub_classes, stub_functions


def _decorators(node: ast.AST) -> set[str]:
    names = set()
    for dec in getattr(node, "decorator_list", []):
        if isinstance(dec, ast.Name):
            names.add(dec.id)
        elif isinstance(dec, ast.Attribute):
            names.add(dec.attr)
    return names


def _stub_default(node: ast.expr):
    try:
        return ast.literal_eval(node)
    except ValueError:
        return ast.unparse(node)


def _stub_params(fn: ast.FunctionDef, drop_first: bool):
    """[(name, kind, default)] of a stub function (kind as in inspect)."""
    a = fn.args
    positional = a.posonlyargs + a.args
    defaults = [_MISSING] * (len(positional) - len(a.defaults)) + list(a.defaults)
    params = []
    for arg, default in zip(positional, defaults):
        kind = (
            inspect.Parameter.POSITIONAL_ONLY
            if arg in a.posonlyargs
            else inspect.Parameter.POSITIONAL_OR_KEYWORD
        )
        params.append(
            (arg.arg, kind, _MISSING if default is _MISSING else _stub_default(default))
        )
    if a.vararg is not None:
        params.append((a.vararg.arg, inspect.Parameter.VAR_POSITIONAL, _MISSING))
    for arg, default in zip(a.kwonlyargs, a.kw_defaults):
        params.append(
            (
                arg.arg,
                inspect.Parameter.KEYWORD_ONLY,
                _MISSING if default is None else _stub_default(default),
            )
        )
    if a.kwarg is not None:
        params.append((a.kwarg.arg, inspect.Parameter.VAR_KEYWORD, _MISSING))
    if drop_first and params:
        params = params[1:]
    return params


def _runtime_params(obj, drop_first: bool):
    try:
        sig = inspect.signature(obj)
    except (TypeError, ValueError):
        return None
    params = []
    for p in sig.parameters.values():
        default = _MISSING if p.default is inspect.Parameter.empty else p.default
        params.append((p.name, p.kind, default))
    if drop_first and params and params[0][0] in ("self", "$self"):
        params = params[1:]
    return params


def _compare(where: str, runtime, stub):
    rt_names = [(n, k) for n, k, _ in runtime]
    st_names = [(n, k) for n, k, _ in stub]
    assert rt_names == st_names, (
        f"{where}: parameters differ\n  runtime: {rt_names}\n  stub:    {st_names}"
    )
    for (name, _, rt_default), (_, _, st_default) in zip(runtime, stub):
        if rt_default is _MISSING:
            assert st_default is _MISSING, f"{where}: '{name}' has no default at runtime"
            continue
        assert st_default is not _MISSING, f"{where}: '{name}' needs default {rt_default!r}"
        if rt_default is Ellipsis:
            continue  # PyO3 could not render the default; any stub default is fine
        if isinstance(rt_default, float) and isinstance(st_default, (int, float)):
            assert float(st_default) == rt_default, (
                f"{where}: default of '{name}' is {rt_default!r} at runtime, "
                f"{st_default!r} in the stub"
            )
        else:
            assert st_default == rt_default, (
                f"{where}: default of '{name}' is {rt_default!r} at runtime, "
                f"{st_default!r} in the stub"
            )


def _public_native_names():
    return [n for n in dir(native) if not n.startswith("_")]


def test_native_classes_and_functions_in_stub():
    stub_classes, stub_functions = _parse_stub()

    for name in _public_native_names():
        obj = getattr(native, name)
        if inspect.isclass(obj):
            assert name in stub_classes, f"class {name} missing from _native.pyi"
        else:
            assert name in stub_functions, f"function {name} missing from _native.pyi"


def test_native_class_members_in_stub():
    stub_classes, _ = _parse_stub()

    for name in _public_native_names():
        obj = getattr(native, name)
        if not inspect.isclass(obj):
            continue
        stub_members = stub_classes.get(name, {})
        for attr in dir(obj):
            if attr.startswith("_"):
                continue
            assert attr in stub_members, (
                f"{name}.{attr} exposed by the native module but missing "
                f"from _native.pyi — update the stub in the same PR"
            )


def test_stub_has_no_members_the_module_lacks():
    """The stub must not advertise names the bindings do not provide."""
    stub_classes, stub_functions = _parse_stub()
    for cls_name, members in stub_classes.items():
        if cls_name in ("Illumination",):
            continue
        assert hasattr(native, cls_name), f"stub class {cls_name} not in the module"
        cls = getattr(native, cls_name)
        for member in members:
            if member.startswith("__"):
                continue
            assert hasattr(cls, member), f"stub member {cls_name}.{member} not in the module"
    for fn in stub_functions:
        assert hasattr(native, fn), f"stub function {fn} not in the module"


def test_native_member_kinds_match_stub():
    """Properties are @property, static methods @staticmethod, methods plain."""
    stub_classes, _ = _parse_stub()
    for name in _public_native_names():
        cls = getattr(native, name)
        if not inspect.isclass(cls):
            continue
        for attr in dir(cls):
            if attr.startswith("_"):
                continue
            raw = inspect.getattr_static(cls, attr)
            node = stub_classes[name][attr]
            decorators = _decorators(node)
            where = f"{name}.{attr}"
            if isinstance(raw, staticmethod) or type(raw).__name__ == "staticmethod":
                assert "staticmethod" in decorators, f"{where} should be @staticmethod"
            elif inspect.isgetsetdescriptor(raw) or inspect.ismemberdescriptor(raw):
                assert isinstance(node, ast.AnnAssign) or "property" in decorators, (
                    f"{where} is a property at runtime"
                )
            elif isinstance(raw, cls):
                assert isinstance(node, ast.AnnAssign), f"{where} is a class attribute"
            else:
                assert isinstance(node, ast.FunctionDef), f"{where} is a method at runtime"
                assert not decorators & {"property", "staticmethod"}, (
                    f"{where} is a plain method at runtime"
                )


def test_native_signatures_match_stub():
    """Parameter names, order, kinds and defaults agree with the bindings."""
    stub_classes, stub_functions = _parse_stub()
    for name in _public_native_names():
        obj = getattr(native, name)
        if inspect.isclass(obj):
            members = stub_classes[name]
            if obj.__text_signature__ is not None and "__init__" in members:
                runtime = _runtime_params(obj, drop_first=False)
                stub = _stub_params(members["__init__"], drop_first=True)
                _compare(f"{name}.__init__", runtime, stub)
            for attr in dir(obj):
                if attr.startswith("_"):
                    continue
                raw = inspect.getattr_static(obj, attr)
                node = members.get(attr)
                if not isinstance(node, ast.FunctionDef) or "property" in _decorators(node):
                    continue
                is_static = "staticmethod" in _decorators(node)
                runtime = _runtime_params(getattr(obj, attr), drop_first=not is_static)
                if runtime is None:
                    continue
                if isinstance(raw, staticmethod) or is_static:
                    stub = _stub_params(node, drop_first=False)
                else:
                    stub = _stub_params(node, drop_first=True)
                _compare(f"{name}.{attr}", runtime, stub)
        else:
            runtime = _runtime_params(obj, drop_first=False)
            if runtime is None:
                continue
            _compare(name, runtime, _stub_params(stub_functions[name], drop_first=False))
