#!/usr/bin/env python3
"""Translate the Blade Lab's signature shaders to WGSL for the game.

A signature is one blade's own shader from Creyon's Blade Lab (a three.js page, GLSL ES 1.0):
`sig_glow`, `sig_core` and `sig_solid` and their helpers, written against the lab's helper
library (`lab.glsl`) and uniforms. The GLSL in `crates/sjk-viewer/src/blade_signatures/` is
the source; this writes the `.wgsl` beside each `.glsl` (and `lab.wgsl` from `lab.glsl`),
which `blade_signatures.rs` builds into the blade's pipelines.

    python3 scripts/blade_signatures.py          # write every .wgsl
    python3 scripts/blade_signatures.py --check  # fail if one is out of date

The translator reads the lab's subset (functions, constants, declarations, if/for/while and
the usual expressions), infers every expression's type from the declarations and writes
readable WGSL: GLSL's scalar arguments to vector builtins are splatted, `mod` becomes a
floor modulo, `?:` becomes `select`, `out` parameters become pointers, assigned parameters
local copies and multi-component swizzle assignments one assignment per component. Names
that `saber.wgsl` or WGSL already take get a `sig_` prefix (functions) or a trailing `_`.
"""
import re
from pathlib import Path
import sys

KEYWORDS_WGSL = set("""
alias break case const const_assert continue continuing default diagnostic discard else enable
false fn for if let loop override requires return struct switch true var while
NULL Self abstract active alignas alignof as asm asm_fragment async attribute auto await become
cast catch class co_await co_return co_yield coherent column_major common compile
compile_fragment concept const_cast consteval constexpr constinit crate debugger decltype delete
demote demote_to_helper do dynamic_cast enum explicit export extends extern external fallthrough
filter final finally friend from fxgroup get goto groupshared highp impl implements import inline
instanceof interface layout lowp macro macro_rules match mediump meta mod module move mut
mutable namespace new nil noexcept noinline nointerpolation non_coherent noncoherent
noperspective null nullptr of operator package packoffset partition pass patch pixelfragment
precise precision premerge priv protected pub public readonly ref regardless register
reinterpret_cast require resource restrict self set shared sizeof smooth snorm static
static_assert static_cast std subroutine super target template this thread_local throw trait
try type typedef typeid typename typeof union unless unorm unsafe unsized use using varying
virtual volatile wgsl where with writeonly yield
""".split())

TYPES = {
    "float": "f32", "int": "i32", "bool": "bool",
    "vec2": "vec2<f32>", "vec3": "vec3<f32>", "vec4": "vec4<f32>",
    "ivec2": "vec2<i32>", "ivec3": "vec3<i32>", "ivec4": "vec4<i32>",
    "bvec2": "vec2<bool>", "bvec3": "vec3<bool>", "bvec4": "vec4<bool>",
    "mat2": "mat2x2<f32>", "mat3": "mat3x3<f32>", "mat4": "mat4x4<f32>",
    "void": None,
}
VEC_N = {"vec2": 2, "vec3": 3, "vec4": 4, "ivec2": 2, "ivec3": 3, "ivec4": 4,
         "bvec2": 2, "bvec3": 3, "bvec4": 4}
SCALAR_OF = {"vec2": "float", "vec3": "float", "vec4": "float",
             "ivec2": "int", "ivec3": "int", "ivec4": "int",
             "bvec2": "bool", "bvec3": "bool", "bvec4": "bool"}


def vec_of(scalar, n):
    if n == 1:
        return scalar
    return {"float": "vec", "int": "ivec", "bool": "bvec"}[scalar] + str(n)


def scalar(t):
    return SCALAR_OF.get(t, t)


def width(t):
    return VEC_N.get(t, 1)


class Error(Exception):
    pass


# ---------------------------------------------------------------- tokens

TOKEN = re.compile(r"""
    (?P<ws>\s+|//[^\n]*|/\*.*?\*/)
  | (?P<num>(?:\d+\.\d*|\.\d+|\d+)(?:[eE][-+]?\d+)?)
  | (?P<id>[A-Za-z_]\w*)
  | (?P<op>\+\+|--|\+=|-=|\*=|/=|==|!=|<=|>=|&&|\|\||[-+*/<>=!?:;,.(){}\[\]])
""", re.S | re.X)


def tokenize(src):
    """Tokens, and for each token's index the comment lines just before it."""
    out = []
    comments = {}
    pending = []
    pos = 0
    while pos < len(src):
        m = TOKEN.match(src, pos)
        if not m:
            raise Error(f"cannot read {src[pos:pos + 20]!r}")
        pos = m.end()
        if m.lastgroup == "ws":
            text = m.group()
            if text.startswith("//"):
                body = text[2:].strip()
                if not body.startswith("@"):
                    pending.append(body)
            elif text.startswith("/*"):
                pending.extend(l.strip(" *") for l in text[2:-2].splitlines() if l.strip(" *"))
            continue
        if pending:
            comments[len(out)] = pending
            pending = []
        out.append((m.lastgroup, m.group()))
    out.append(("eof", ""))
    return out, comments


# ---------------------------------------------------------------- parser (AST as tuples/dicts)

BINARY = {
    "||": 1, "&&": 2, "==": 3, "!=": 3, "<": 4, ">": 4, "<=": 4, ">=": 4,
    "+": 5, "-": 5, "*": 6, "/": 6,
}


class Parser:
    def __init__(self, tokens):
        self.t, self.comments = tokens
        self.i = 0

    def take_comments(self):
        return self.comments.pop(self.i, [])

    def peek(self, k=0):
        return self.t[self.i + k]

    def next(self):
        tok = self.t[self.i]
        self.i += 1
        return tok

    def accept(self, value):
        if self.peek()[1] == value and self.peek()[0] in ("op", "id"):
            self.i += 1
            return True
        return False

    def expect(self, value):
        tok = self.next()
        if tok[1] != value:
            raise Error(f"expected {value!r}, found {tok[1]!r} near {self.context()}")
        return tok

    def context(self):
        return " ".join(v for _, v in self.t[max(0, self.i - 8):self.i + 4])

    def is_type(self, k=0):
        kind, value = self.peek(k)
        return kind == "id" and value in TYPES

    # top level
    def unit(self):
        items = []
        while self.peek()[0] != "eof":
            notes = self.take_comments()
            if notes:
                items.append(("notes", notes))
            if self.accept("precision"):
                while not self.accept(";"):
                    self.next()
                continue
            const = self.accept("const")
            qual = None
            if self.peek()[1] in ("uniform", "varying", "attribute"):
                raise Error("uniforms in a signature are not supported")
            ty = self.next()[1]
            if ty not in TYPES:
                raise Error(f"unknown type {ty!r} near {self.context()}")
            name = self.next()[1]
            if self.peek()[1] == "(":
                items.append(self.function(ty, name))
            else:
                decls = self.declarators(ty, name)
                items.append(("global", const, ty, decls))
                self.expect(";")
        return items

    def function(self, ty, name):
        self.expect("(")
        params = []
        if not self.accept(")"):
            while True:
                qual = "in"
                if self.peek()[1] in ("in", "out", "inout"):
                    qual = self.next()[1]
                if self.peek()[1] == "const":
                    self.next()
                pty = self.next()[1]
                if pty == "void" and self.peek()[1] == ")":
                    break
                pname = self.next()[1]
                params.append((qual, pty, pname))
                if self.accept(")"):
                    break
                self.expect(",")
        if self.accept(";"):
            return ("prototype", ty, name, params)
        body = self.block()
        return ("function", ty, name, params, body)

    def declarators(self, ty, name):
        decls = []
        while True:
            init = None
            if self.accept("="):
                init = self.expr(no_comma=True)
            decls.append((name, init))
            if not self.accept(","):
                break
            name = self.next()[1]
        return decls

    def block(self):
        self.expect("{")
        stmts = []
        while not self.accept("}"):
            stmts.append(self.statement())
        return ("block", stmts)

    def statement(self):
        notes = self.take_comments()
        s = self.bare_statement()
        return ("noted", notes, s) if notes else s

    def bare_statement(self):
        tok = self.peek()
        v = tok[1]
        if v == "{":
            return self.block()
        if v == ";":
            self.next()
            return ("empty",)
        if v == "if":
            self.next()
            self.expect("(")
            cond = self.expr()
            self.expect(")")
            then = self.statement()
            other = None
            if self.accept("else"):
                other = self.statement()
            return ("if", cond, then, other)
        if v == "for":
            self.next()
            self.expect("(")
            init = None
            if not self.accept(";"):
                init = self.simple_statement()
                self.expect(";")
            cond = None
            if not self.accept(";"):
                cond = self.expr()
                self.expect(";")
            step = None
            if self.peek()[1] != ")":
                step = self.expr()
            self.expect(")")
            body = self.statement()
            return ("for", init, cond, step, body)
        if v == "while":
            self.next()
            self.expect("(")
            cond = self.expr()
            self.expect(")")
            return ("while", cond, self.statement())
        if v == "return":
            self.next()
            if self.accept(";"):
                return ("return", None)
            e = self.expr()
            self.expect(";")
            return ("return", e)
        if v in ("break", "continue", "discard"):
            self.next()
            self.expect(";")
            return (v,)
        s = self.simple_statement()
        self.expect(";")
        return s

    def simple_statement(self):
        const = False
        if self.peek()[1] == "const":
            self.next()
            const = True
        if self.is_type() and self.peek(1)[0] == "id":
            ty = self.next()[1]
            name = self.next()[1]
            return ("decl", const, ty, self.declarators(ty, name))
        return ("expr", self.expr())

    # expressions
    def expr(self, no_comma=False):
        e = self.assignment()
        if not no_comma and self.peek()[1] == ",":
            seq = [e]
            while self.accept(","):
                seq.append(self.assignment())
            return ("seq", seq)
        return e

    def assignment(self):
        left = self.ternary()
        if self.peek()[1] in ("=", "+=", "-=", "*=", "/="):
            op = self.next()[1]
            right = self.assignment()
            return ("assign", op, left, right)
        return left

    def ternary(self):
        cond = self.binary(1)
        if self.accept("?"):
            a = self.assignment()
            self.expect(":")
            b = self.assignment()
            return ("ternary", cond, a, b)
        return cond

    def binary(self, level):
        left = self.unary()
        while True:
            op = self.peek()[1]
            prec = BINARY.get(op) if self.peek()[0] == "op" else None
            if prec is None or prec < level:
                return left
            self.next()
            right = self.binary(prec + 1)
            left = ("bin", op, left, right)

    def unary(self):
        v = self.peek()[1]
        if self.peek()[0] == "op" and v in ("-", "+", "!"):
            self.next()
            operand = self.unary()
            if v == "+":
                return operand
            return ("un", v, operand)
        if self.peek()[0] == "op" and v in ("++", "--"):
            self.next()
            return ("pre", v, self.unary())
        return self.postfix(self.primary())

    def postfix(self, e):
        while True:
            v = self.peek()[1]
            if v == "." and self.peek()[0] == "op":
                self.next()
                e = ("member", e, self.next()[1])
            elif v == "[":
                self.next()
                idx = self.expr()
                self.expect("]")
                e = ("index", e, idx)
            elif v in ("++", "--"):
                self.next()
                e = ("post", v, e)
            else:
                return e

    def primary(self):
        kind, v = self.next()
        if kind == "num":
            return ("num", v)
        if kind == "id":
            if v in ("true", "false"):
                return ("bool", v)
            if self.peek()[1] == "(":
                self.next()
                args = []
                if not self.accept(")"):
                    while True:
                        args.append(self.assignment())
                        if self.accept(")"):
                            break
                        self.expect(",")
                return ("call", v, args)
            return ("id", v)
        if v == "(":
            e = self.expr()
            self.expect(")")
            return ("paren", e)
        raise Error(f"unexpected {v!r} near {self.context()}")


# ---------------------------------------------------------------- types of builtins

GEN_SAME = {  # result is the type of the first argument (genType)
    "sin", "cos", "tan", "asin", "acos", "atan", "exp", "log", "exp2", "log2", "sqrt",
    "inversesqrt", "abs", "sign", "floor", "ceil", "fract", "radians", "degrees", "normalize",
    "pow", "mod", "min", "max", "clamp", "mix", "step", "smoothstep", "reflect",
}
SCALAR_RESULT = {"length": "float", "distance": "float", "dot": "float"}


class Translator:
    def __init__(self, globals_, functions, rename, private_names):
        self.globals = dict(globals_)  # name -> type
        self.functions = dict(functions)  # name -> (ret, params)
        self.rename = dict(rename)  # GLSL name -> WGSL name (module-level)
        self.private = set(private_names)
        self.out_lines = []

    # ---- names
    def local_name(self, name):
        if name in KEYWORDS_WGSL or name.startswith("__") or name in self.module_names:
            return name + "_"
        return name

    # ---- scope
    def lookup(self, name):
        for scope in reversed(self.scopes):
            if name in scope:
                return scope[name]
        if name in self.globals:
            return {"type": self.globals[name], "wgsl": self.rename.get(name, name), "ptr": False}
        raise Error(f"unknown name {name!r} in {self.fn_name}")

    # ---- types
    def type_of(self, e):
        k = e[0]
        if k == "num":
            v = e[1]
            return "float" if ("." in v or "e" in v.lower()) else "int"
        if k == "bool":
            return "bool"
        if k == "id":
            return self.lookup(e[1])["type"]
        if k == "paren":
            return self.type_of(e[1])
        if k == "seq":
            return self.type_of(e[1][-1])
        if k == "assign":
            return self.type_of(e[2])
        if k == "ternary":
            a, b = self.type_of(e[2]), self.type_of(e[3])
            return a if width(a) >= width(b) else b
        if k in ("un", "pre", "post"):
            if e[1] == "!":
                return "bool"
            return self.type_of(e[2])
        if k == "bin":
            op = e[1]
            if op in ("==", "!=", "<", ">", "<=", ">=", "&&", "||"):
                return "bool"
            a, b = self.type_of(e[2]), self.type_of(e[3])
            if a.startswith("mat") and b.startswith("mat"):
                return a
            if a.startswith("mat") and op == "*":
                return b if b in VEC_N else a
            if b.startswith("mat") and op == "*":
                return a if a in VEC_N else b
            if width(a) >= width(b):
                return a if width(a) > 1 or scalar(b) != "float" else (b if width(b) > 1 else ("float" if "float" in (a, b) else a))
            return b
        if k == "member":
            base = self.type_of(e[1])
            n = len(e[2])
            return vec_of(scalar(base), n)
        if k == "index":
            base = self.type_of(e[1])
            if base.startswith("mat"):
                return "vec" + base[3]
            return scalar(base)
        if k == "call":
            name, args = e[1], e[2]
            if name in TYPES:
                return name
            if name in self.functions:
                return self.functions[name][0]
            if name in SCALAR_RESULT:
                return SCALAR_RESULT[name]
            if name == "cross" or name == "skinBody":
                return "vec3"
            if name in GEN_SAME:
                types = [self.type_of(a) for a in args]
                if name in ("step",):
                    return max(types, key=width)
                if name in ("smoothstep",):
                    return max(types, key=width)
                return max(types, key=width)
            raise Error(f"unknown function {name!r} in {self.fn_name}")
        raise Error(f"no type for {k}")

    # ---- expressions
    PREC = {"||": 1, "&&": 2, "==": 3, "!=": 3, "<": 4, ">": 4, "<=": 4, ">=": 4,
            "+": 5, "-": 5, "*": 6, "/": 6}

    def num(self, v, want_float):
        if want_float or "." in v or "e" in v.lower():
            if "e" in v.lower():
                mant, exp = re.split("[eE]", v)
                if "." not in mant:
                    mant += ".0"
                if mant.startswith("."):
                    mant = "0" + mant
                if mant.endswith("."):
                    mant += "0"
                return mant + "e" + exp
            if "." not in v:
                return v + ".0"
            if v.startswith("."):
                v = "0" + v
            if v.endswith("."):
                v += "0"
            return v
        return v

    def as_type(self, e, want, prec=0):
        """`e` emitted as type `want` (a float literal for an int literal; a splat for a scalar
        given where a vector is wanted)."""
        t = self.type_of(e)
        if want == "float" and e[0] == "num":
            return self.num(e[1], True)
        if width(want) > 1 and width(t) == 1:
            return f"{self.ctor_name(want)}({self.as_type(e, scalar(want))})"
        if scalar(want) == "float" and scalar(t) == "int" and width(want) == width(t):
            return f"{self.ctor_name(want)}({self.emit(e)})"
        return self.emit(e, prec)

    def ctor_name(self, t):
        if t == "float":
            return "f32"
        if t == "int":
            return "i32"
        if t in VEC_N:
            n = VEC_N[t]
            s = scalar(t)
            return f"vec{n}" if s == "float" else f"vec{n}<{TYPES[s]}>"
        return TYPES[t]

    def emit(self, e, prec=0):
        k = e[0]
        if k == "num":
            return self.num(e[1], False)
        if k == "bool":
            return e[1]
        if k == "id":
            info = self.lookup(e[1])
            return f"(*{info['wgsl']})" if info["ptr"] else info["wgsl"]
        if k == "paren":
            inner = e[1]
            # keep the source's grouping where it changes meaning; drop it around atoms
            if inner[0] in ("num", "id", "call", "member", "paren", "bool", "index"):
                return self.emit(inner, prec)
            return "(" + self.emit(inner, 0) + ")"
        if k == "un":
            op = e[1]
            inner = self.emit(e[2], 7)
            if op == "-" and inner.startswith("-"):
                inner = "(" + inner + ")"
            return op + inner
        if k == "bin":
            op = e[1]
            p = self.PREC[op]
            ta, tb = self.type_of(e[2]), self.type_of(e[3])
            want = None
            if op in ("+", "-", "*", "/") and "float" in (scalar(ta), scalar(tb)):
                want = "float"
            # WGSL forbids mixing && and || (and chaining comparisons) without parentheses
            def side(x, right):
                if want == "float" and x[0] == "num":
                    return self.num(x[1], True)
                s = self.emit(x, p + (1 if right else 0))
                if x[0] == "bin" and ((op in ("&&", "||") and x[1] in ("&&", "||") and x[1] != op)
                                      or (p in (3, 4) and self.PREC[x[1]] in (3, 4))):
                    if not s.startswith("("):
                        s = "(" + s + ")"
                return s
            text = f"{side(e[2], False)} {op} {side(e[3], True)}"
            return f"({text})" if p < prec else text
        if k == "ternary":
            t = self.type_of(e)
            a = self.as_type(e[2], t)
            b = self.as_type(e[3], t)
            return f"select({b}, {a}, {self.emit(e[1])})"
        if k == "member":
            return f"{self.emit(e[1], 8)}.{e[2]}"
        if k == "index":
            return f"{self.emit(e[1], 8)}[{self.emit(e[2])}]"
        if k == "call":
            return self.call(e)
        if k == "post" or k == "pre":
            raise Error("++/-- inside an expression")
        if k == "assign":
            raise Error("assignment inside an expression")
        if k == "seq":
            raise Error("comma expression inside an expression")
        raise Error(f"cannot emit {k}")

    def call(self, e):
        name, args = e[1], e[2]
        if name in TYPES:  # constructor
            if name in ("float", "int", "bool"):
                return f"{self.ctor_name(name)}({self.emit(args[0])})"
            s = scalar(name)
            parts = []
            for a in args:
                ta = self.type_of(a)
                if scalar(ta) != s and not name.startswith("mat"):
                    parts.append(f"{self.ctor_name(vec_of(s, width(ta)))}({self.emit(a)})")
                elif s == "float" and a[0] == "num":
                    parts.append(self.num(a[1], True))
                else:
                    parts.append(self.emit(a))
            ctor = self.ctor_name(name)
            if self.constant(e) and ctor.startswith("vec") and "<" not in ctor:
                ctor += "<f32>"  # all literals: say the type, or WGSL keeps it abstract
            return f"{ctor}({', '.join(parts)})"
        if name in self.functions:
            ret, params = self.functions[name]
            parts = []
            for (qual, pty, _), a in zip(params, args):
                if qual in ("out", "inout"):
                    if a[0] != "id":
                        raise Error(f"out argument {a} to {name} is not a variable")
                    info = self.lookup(a[1])
                    parts.append(info["wgsl"] if info["ptr"] else "&" + info["wgsl"])
                    info["assigned"] = True
                else:
                    parts.append(self.as_type(a, pty))
            return f"{self.rename.get(name, name)}({', '.join(parts)})"
        types = [self.type_of(a) for a in args]
        if name == "mod":
            t = max(types, key=width)
            ty = scalar(t) if width(types[1]) == 1 else t
            x = self.as_type(args[0], t)
            y = self.as_type(args[1], ty)
            helper = "glsl_mod" if t == "float" else f"glsl_mod_{t}" + ("" if ty == t else "_" + ty)
            self.mods[helper] = (TYPES[t], TYPES[ty])
            return f"{helper}({x}, {y})"
        if name == "atan" and len(args) == 2:
            return f"atan2({self.as_type(args[0], 'float' if width(types[0]) == 1 else types[0])}, {self.as_type(args[1], types[0])})"
        if name == "inversesqrt":
            return f"inverseSqrt({self.emit(args[0])})"
        if name in ("min", "max", "clamp", "pow", "step", "smoothstep", "mix", "distance",
                    "dot", "reflect"):
            t = max(types, key=width)
            if name == "mix":
                parts = [self.as_type(args[0], t), self.as_type(args[1], t),
                         self.as_type(args[2], types[2] if width(types[2]) == 1 else t)]
                if width(types[2]) == 1:
                    parts[2] = self.as_type(args[2], "float")
            else:
                parts = [self.as_type(a, t) for a in args]
            return f"{name}({', '.join(parts)})"
        if name in GEN_SAME or name in SCALAR_RESULT or name == "cross":
            parts = [self.as_type(a, "float") if self.type_of(a) == "int" and name not in ("abs", "sign", "min", "max", "clamp") else self.emit(a) for a in args]
            return f"{name}({', '.join(parts)})"
        if name == "skinBody":
            return f"skinBody({self.as_type(args[0], 'vec2')})"
        raise Error(f"unknown function {name}")

    # ---- statements
    def line(self, text):
        self.out_lines.append("    " * self.depth + text)

    def assigned_names(self, node, acc):
        if isinstance(node, tuple):
            if len(node) == 4 and node[0] == "assign":
                target = node[2]
                while target[0] in ("member", "index", "paren"):
                    target = target[1]
                if target[0] == "id":
                    acc.add(target[1])
            if len(node) == 3 and node[0] in ("post", "pre"):
                target = node[2]
                while target[0] in ("member", "index", "paren"):
                    target = target[1]
                if target[0] == "id":
                    acc.add(target[1])
            if len(node) == 3 and node[0] == "call" and node[1] in self.functions:
                for (qual, _, _), a in zip(self.functions[node[1]][1], node[2]):
                    if qual in ("out", "inout") and a[0] == "id":
                        acc.add(a[1])
            for x in node:
                self.assigned_names(x, acc)
        elif isinstance(node, list):
            for x in node:
                self.assigned_names(x, acc)
        elif isinstance(node, dict):
            pass
        return acc

    def function(self, item):
        _, ret, name, params, body = item
        self.fn_name = name
        self.depth = 0
        self.scopes = [{}]
        assigned = self.assigned_names(body, set())
        parts = []
        copies = []
        for qual, pty, pname in params:
            wname = self.local_name(pname)
            if qual in ("out", "inout"):
                parts.append(f"{wname}: ptr<function, {TYPES[pty]}>")
                self.scopes[0][pname] = {"type": pty, "wgsl": wname, "ptr": True}
            elif pname in assigned:
                parts.append(f"{wname}_in: {TYPES[pty]}")
                copies.append(f"var {wname} = {wname}_in;")
                self.scopes[0][pname] = {"type": pty, "wgsl": wname, "ptr": False}
            else:
                parts.append(f"{wname}: {TYPES[pty]}")
                self.scopes[0][pname] = {"type": pty, "wgsl": wname, "ptr": False}
        self.ret = ret
        result = f" -> {TYPES[ret]}" if TYPES[ret] else ""
        self.line(f"fn {self.rename.get(name, name)}({', '.join(parts)}){result} {{")
        self.depth += 1
        for c in copies:
            self.line(c)
        self.fn_assigned = assigned
        for s in body[1]:
            self.statement(s)
        self.depth -= 1
        self.line("}")

    def declare(self, const, ty, decls, module=False):
        for name, init in decls:
            wname = self.local_name(name) if not module else self.rename.get(name, name)
            mutable = name in self.fn_assigned or init is None
            if not module:
                # a local shadowing an outer one of the same name keeps WGSL happy (new scope)
                self.scopes[-1][name] = {"type": ty, "wgsl": wname, "ptr": False}
            if init is None:
                self.line(f"var {wname}: {TYPES[ty]};")
                continue
            value = self.as_type(init, ty)
            kw = "var" if mutable else "let"
            self.line(f"{kw} {wname}: {TYPES[ty]} = {value};" if self.needs_type(init, ty) else f"{kw} {wname} = {value};")

    def needs_type(self, init, ty):
        # spelled out where the value alone would not say it (a literal, a converted value)
        return self.type_of(init) != ty or self.constant(init)

    def constant(self, e):
        """Whether `e` has no names in it (literals only): its WGSL type would be abstract."""
        if isinstance(e, tuple):
            if e and e[0] == "id":
                return False
            if e and e[0] == "call" and e[1] not in TYPES:
                return False
            return all(self.constant(x) for x in e[1:] if isinstance(x, (tuple, list)))
        if isinstance(e, list):
            return all(self.constant(x) for x in e)
        return True

    def statement(self, s):
        k = s[0]
        if k == "noted":
            for note in s[1]:
                self.line(f"// {note}" if note else "//")
            self.statement(s[2])
        elif k == "block":
            self.line("{")
            self.depth += 1
            self.scopes.append({})
            for x in s[1]:
                self.statement(x)
            self.scopes.pop()
            self.depth -= 1
            self.line("}")
        elif k == "empty":
            pass
        elif k == "decl":
            self.declare(s[1], s[2], s[3])
        elif k == "expr":
            self.expr_statement(s[1])
        elif k == "if":
            self.line(f"if {self.cond(s[1])} {{")
            self.body(s[2])
            other = s[3]
            while other is not None and other[0] == "if":
                self.line(f"}} else if {self.cond(other[1])} {{")
                self.body(other[2])
                other = other[3]
            if other is not None:
                self.line("} else {")
                self.body(other)
            self.line("}")
        elif k == "for":
            init, cond, step, body = s[1:]
            self.scopes.append({})
            init_text = ""
            if init is not None:
                if init[0] == "decl":
                    (name, value), = init[3]
                    wname = self.local_name(name)
                    self.scopes[-1][name] = {"type": init[2], "wgsl": wname, "ptr": False}
                    init_text = f"var {wname} = {self.as_type(value, init[2])}" if init[2] != "float" else f"var {wname}: f32 = {self.as_type(value, 'float')}"
                else:
                    init_text = self.simple(init[1])
            cond_text = self.emit(cond) if cond is not None else ""
            step_text = self.simple(step) if step is not None else ""
            self.line(f"for ({init_text}; {cond_text}; {step_text}) {{")
            self.body(body)
            self.line("}")
            self.scopes.pop()
        elif k == "while":
            self.line(f"while {self.cond(s[1])} {{")
            self.body(s[2])
            self.line("}")
        elif k == "return":
            if s[1] is None:
                self.line("return;")
            else:
                self.line(f"return {self.as_type(s[1], self.ret)};")
        elif k in ("break", "continue", "discard"):
            self.line(f"{k};")
        else:
            raise Error(f"statement {k}")

    def cond(self, e):
        return self.emit(e)

    def body(self, s):
        self.depth += 1
        self.scopes.append({})
        if s[0] == "block":
            for x in s[1]:
                self.statement(x)
        else:
            self.statement(s)
        self.scopes.pop()
        self.depth -= 1

    def simple(self, e):
        """A for loop's init or step, or a statement's expression, as one WGSL statement."""
        k = e[0]
        if k in ("post", "pre"):
            return f"{self.emit(e[2])}{e[1]}"
        if k == "assign":
            lines = self.assignment(e)
            if len(lines) != 1:
                raise Error("a split assignment in a for loop")
            return lines[0]
        if k == "seq":
            raise Error("comma expression")
        return "_ = " + self.emit(e)

    def assignment(self, e):
        _, op, target, value = e
        ttype = self.type_of(target)
        if target[0] == "member" and len(target[2]) > 1:
            base = self.emit(target[1], 8)
            comps = target[2]
            vtype = self.type_of(value)
            lines = []
            if value[0] in ("id", "num") or width(vtype) == 1:
                for i, c in enumerate(comps):
                    v = self.as_type(value, "float") if width(vtype) == 1 else f"{self.emit(value, 8)}.{'xyzw'[i]}"
                    lines.append(f"{base}.{c} {op} {v};")
            else:
                tmp = f"sig_t{self.temp}"
                self.temp += 1
                lines.append(f"let {tmp} = {self.emit(value)};")
                for i, c in enumerate(comps):
                    lines.append(f"{base}.{c} {op} {tmp}.{'xyzw'[i]};")
            return lines
        want = ttype
        if op in ("*=", "/=") and width(self.type_of(value)) == 1:
            want = scalar(ttype)
        if op == "*=" and self.type_of(value).startswith("mat"):
            return [f"{self.emit(target)} = {self.emit(target, 6)} * {self.emit(value, 7)};"]
        return [f"{self.emit(target)} {op} {self.as_type(value, want)};"]

    def expr_statement(self, e):
        if e[0] == "seq":
            for x in e[1]:
                self.expr_statement(x)
            return
        if e[0] == "assign":
            for l in self.assignment(e):
                self.line(l)
            return
        if e[0] in ("post", "pre"):
            self.line(f"{self.emit(e[2])}{e[1]};")
            return
        if e[0] == "call":
            self.line(f"_ = {self.emit(e)};" if self.type_of(e) != "void" else f"{self.emit(e)};")
            return
        raise Error(f"expression statement {e[0]}")


def translate(src, uniforms, helpers=None, module_names=(), prefix_clash="sig_"):
    """WGSL for GLSL `src`. `uniforms` maps the GLSL names read as module-level values to
    their GLSL types; `helpers` is an earlier translation's (globals, functions) to call."""
    items = Parser(tokenize(src)).unit()
    globals_ = dict(uniforms)
    functions = {}
    if helpers:
        globals_.update(helpers[0])
        functions.update(helpers[1])
    rename = {}
    for item in items:
        if item[0] in ("function", "prototype"):
            name = item[2]
            functions[name] = (item[1], item[3])
            if name in module_names or name in KEYWORDS_WGSL:
                rename[name] = prefix_clash + name
        elif item[0] == "global":
            for name, _ in item[3]:
                globals_[name] = item[2]
                if name in module_names or name in KEYWORDS_WGSL:
                    rename[name] = prefix_clash + name
    if helpers:
        rename.update(helpers[2])
    tr = Translator(globals_, functions, rename, uniforms)
    tr.module_names = set(module_names) | set(functions) | set(globals_)
    tr.temp = 0
    tr.mods = {}
    out = []
    notes = ""
    for item in items:
        tr.out_lines = []
        if item[0] == "prototype":
            continue
        if item[0] == "notes":
            notes += "\n".join(f"// {n}" if n else "//" for n in item[1]) + "\n"
            continue
        if item[0] == "global":
            _, const, ty, decls = item
            tr.fn_name = "<module>"
            tr.scopes = [{}]
            tr.depth = 0
            tr.fn_assigned = set()
            for name, init in decls:
                if const:
                    tr.line(f"const {rename.get(name, name)}: {TYPES[ty]} = {tr.as_type(init, ty)};")
                elif init is None:
                    tr.line(f"var<private> {rename.get(name, name)}: {TYPES[ty]};")
                else:
                    # A module variable: each pixel's own, starting from its value.
                    tr.line(f"var<private> {rename.get(name, name)}: {TYPES[ty]} = {tr.as_type(init, ty)};")
        else:
            tr.function(item)
        out.append(notes + "\n".join(tr.out_lines))
        notes = ""
    known = set(helpers[3]) if helpers else set()
    for helper, (tx, ty) in sorted(tr.mods.items()):
        if helper in known:
            continue
        known.add(helper)
        out.append(f"// GLSL's mod: x - y * floor(x / y) (WGSL's % truncates)\n"
                   f"fn {helper}(x: {tx}, y: {ty}) -> {tx} {{\n    return x - y * floor(x / y);\n}}")
    exports = (
        {k: v for k, v in globals_.items() if k not in uniforms},
        functions,
        rename,
        known,
    )
    return "\n\n".join(out) + "\n", exports




# ---------------------------------------------------------------- the repository's signatures

UNIFORMS = {name: "float" for name in
            "uTime uClock uLength uSwing uClash uIgn uAge uSeed uChroma uRoom uReach uFx uOwn uParts uSince".split()}
# Light a signature keeps in its own colours under a chroma (round 12: Between Worlds' duel).
UNIFORMS["uKeep"] = "vec3"
UNIFORMS["uTurnFx"] = "vec3"
UNIFORMS["uTurn"] = "float"
UNIFORMS["uBend"] = "float"
UNIFORMS["uSide"] = "float"
UNIFORMS["uAirCover"] = "float"
UNIFORMS["uClashT"] = "float"
HERE = Path(__file__).resolve().parent.parent
FOLDER = HERE / "crates" / "sjk-viewer" / "src" / "blade_signatures"
SABER = HERE / "crates" / "sjk-viewer" / "src" / "saber.wgsl"


def module_names(saber):
    """The names `saber.wgsl` declares at module scope, which a signature must not reuse:
    all but its stand-in signature, which the signature replaces."""
    begin, end = "// @signature begin\n", "// @signature end\n"
    saber = saber[:saber.index(begin)] + saber[saber.index(end):]
    return set(re.findall(r"^(?:fn|const|var(?:<[^>]*>)?|struct|alias|override)\s+(\w+)", saber, re.M))


def header(glsl, name):
    meta = re.findall(r"^// *@(\w+) *(.*?)\s*$", glsl, re.M)
    lines = [f"// Generated by scripts/blade_signatures.py from {name}: edit that file and run the",
             "// script again."]
    lines += [f"// @{key} {value}" for key, value in meta]
    return "\n".join(lines) + "\n\n"


def build():
    """Every signature's WGSL, by file name: the helpers (`lab.wgsl`) and each blade's."""
    names = module_names(SABER.read_text())
    lib_glsl = (FOLDER / "lab.glsl").read_text()
    lib, helpers = translate(lib_glsl, UNIFORMS, module_names=names)
    out = {"lab.wgsl": header(lib_glsl, "lab.glsl") + lib}
    for path in sorted(FOLDER.glob("*.glsl")):
        if path.name == "lab.glsl":
            continue
        glsl = path.read_text()
        try:
            text, _ = translate(glsl, UNIFORMS, helpers, module_names=names | set(helpers[1]))
        except Error as error:
            raise SystemExit(f"{path.name}: {error}")
        out[path.stem + ".wgsl"] = header(glsl, path.name) + text
    return out


def main():
    check = "--check" in sys.argv[1:]
    stale = []
    for name, text in build().items():
        path = FOLDER / name
        if path.exists() and path.read_text() == text:
            continue
        stale.append(name)
        if not check:
            path.write_text(text)
    if check and stale:
        raise SystemExit("out of date: " + ", ".join(stale))
    print(("out of date: " if check else "written: ") + (", ".join(stale) or "none"))


if __name__ == "__main__":
    main()
