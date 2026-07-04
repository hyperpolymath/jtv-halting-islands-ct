# JtV (Julia the Viper) — proof-of-concept

JtV is a Harvard-architecture language enforced at the grammar level, with
two grammatically-sealed sublanguages:

- **DATA** — an addition-only, total (provably-halting) expression
  language. No recursion, no loops, no conditionals branching on
  unevaluated control. Numbers are exact rationals.
- **CONTROL** — Turing-complete and imperative: assignment, `if`/`else`,
  `while`, `print`, blocks.

A **one-way bridge** (the "diode") lets CONTROL bind a *halted* DATA-expression
result into a control variable. DATA can never invoke control flow.

**Load-bearing property:** membership in the DATA grammar *is* the
termination certificate. If a fragment parses as a data-expression, it
terminates — so checking totality collapses to parsing. Because the
bridge is one-way, a data island contributes no non-termination to the
surrounding program: a data block is a **totality oracle** the control
logic may rely on.

This repo is a proof-of-concept of exactly that claim and nothing more.
It is not a language implementation, not a type checker, not an LSP. See
[LIMITATIONS](#limitations) below for what this specifically does *not*
claim.

## 60-second example

```
# examples/oracle.jtv
begin
  n = 5
  sum = 0
  i = 0
  while i < n do
    begin
      sum = sum plus i     # DATA: addition, guaranteed to halt
      i = i plus 1         # DATA: addition, guaranteed to halt
    end
  print(sum)
end
```

`i < n`, `sum plus i`, and `i plus 1` are DATA expressions — each one is
a totality oracle: it is guaranteed to halt the instant it's evaluated,
because addition and comparison over finite terms have nothing to loop
on. The surrounding `while` is CONTROL: openly Turing-complete. Nothing
in JtV proves *this loop* terminates — it happens to, because `i` counts
up to a fixed `n`, which is ordinary reasoning about this program, not a
guarantee the language gives you.

Running it prints `10`. See [LIMITATIONS](#limitations) for why that is
not the same as JtV proving the whole program halts.

Exact arithmetic, no float drift:

```
# examples/pure_data.jtv
1/3 plus 1/3 added_to 1/3
```

evaluates to exactly `1` — DATA numbers are `fractions.Fraction`, not
floats.

And the diode holding: [`examples/violation.jtv`](examples/violation.jtv)
tries to put an `if`/`then`/`else` on the right-hand side of an
assignment (i.e. inside DATA-grammar position) and is **rejected** —
that rejection is the diode working, not a bug.

## How to run

Requires Python 3.11+ (uses `fractions.Fraction` and PEP 604 union
types). No dependencies to run; `pytest` only for the test suite.

```sh
# Evaluate a standalone DATA expression:
python3 -m jtv.cli examples/pure_data.jtv --mode data      # -> 1

# Parse + execute a CONTROL program:
python3 -m jtv.cli examples/oracle.jtv --mode control      # -> 10

# Watch the diode reject a smuggling attempt:
python3 -m jtv.cli examples/violation.jtv --mode control   # -> DataParseError

# Run the test suite (the negative tests in tests/test_diode_negative.py
# are the heart of this POC):
python3 -m pip install pytest
PYTHONPATH=src python3 -m pytest tests/ -v
```

(`PYTHONPATH=src` is required because this POC deliberately has no
package build step — see LIMITATIONS.)

## Repo layout

- `GRAMMAR.ebnf` — full EBNF for DATA, CONTROL, and the bridge.
- `src/jtv/data_ast.py` / `data_parser.py` / `data_eval.py` — the DATA
  sublanguage: its own AST types, its own parser, its own evaluator.
- `src/jtv/control_ast.py` / `control_parser.py` / `control_eval.py` —
  the CONTROL sublanguage, likewise self-contained.
- `src/jtv/diode.py` — the structural validator; see its docstring for
  how the two enforcement layers (grammar separation + AST-construction
  validation) work together.
- `examples/` — `pure_data.jtv`, `oracle.jtv`, `violation.jtv`.
- `tests/test_diode_negative.py` — the negative tests: the actual point
  of this repository.

## Limitations

**This is a proof-of-concept of one specific claim, deliberately scoped
narrow. It is not a language implementation.**

### 1. Totality-oracle vs. whole-program-halting

JtV proves that *each individual DATA expression* halts — that's the
totality-oracle property, and it's what "membership in the grammar is
the termination certificate" means. **It is not a proof that a whole JtV
program halts.** CONTROL is openly Turing-complete (it has unrestricted
`while`), and a CONTROL program can loop forever exactly as any
imperative language can. Asserting whole-program halting would be the
specific error this README is written to avoid making. What JtV actually
buys you: whatever a CONTROL program's own logic can't get right (like
whether its loop counter is bounded), you at least never have to wonder
whether a *data block embedded in it* is the thing that hangs.

### 2. The green is only as wide as the data language

Addition-only is a maximally narrow total fragment, chosen specifically
because it makes the termination argument nearly free (there is
structurally nothing to loop on). Widening DATA toward recursion or
iteration — even something as modest as bounded `for`-loops or
user-defined functions — would force re-establishing totality from
scratch (e.g. via a termination measure, fuel, or a totality checker
distinct from the parser), and membership-checking would no longer
collapse to parsing. This POC does not attempt that; it demonstrates the
narrowest case where the collapse is clean.

### 3. Prior art

JtV's diode sits in a design space with real neighbours, and a full
prior-art search is **pending** — this repo does not claim novelty, only
that it demonstrates one specific point in that space cleanly:
grammar-membership-as-termination-certificate, a one-way diode bridge,
and an injectable oracle block, all enforced structurally rather than by
a runtime check. Relevant neighbours to compare against once that search
happens:

- **Dhall** — a total, non-Turing-complete configuration language
  explicitly designed to guarantee termination.
- **Starlark** — Python-like but deliberately non-Turing-complete
  (bounded loops, no unbounded recursion), used as a safe embedded
  config/build language.
- **CUE** — a total, order-independent configuration language with its
  own constraint-based evaluation model.
- **Nickel** — a configuration language with gradual typing and contracts,
  another point in the "safe DSL embedded in a larger system" space.
- **Staged / two-level languages** — languages with an explicit
  compile-time/run-time (or "meta"/"object") stage separation, which is
  the closest general concept to JtV's Harvard-style DATA/CONTROL split.
- **Effect and capability systems** — type-system mechanisms that
  restrict what a piece of code can do (e.g. "this code cannot perform
  I/O" or "this code cannot diverge"), which is one lens for describing
  what the diode is doing structurally rather than via types.

### Other explicit non-goals of this repo

- No package build/distribution step, no LSP, no richer data language
  than the grammar in `GRAMMAR.ebnf`, no CI beyond running the test
  suite locally. Adding any of these is out of scope for this POC.

## License

See [LICENSE](LICENSE) — placeholder, to be replaced.
