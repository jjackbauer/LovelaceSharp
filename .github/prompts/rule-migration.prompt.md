````prompt
---
agent: plan
description: Migration-specific rule — produce requirements for one C++ class being migrated to C#.
---

#file:.github/prompts/workflow-requirements-gathering.prompt.md
#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/skill-use-digit-store.prompt.md
#file:.github/prompts/skill-impl-completeness.prompt.md

# Rule: Migration Requirements

## Purpose
Pre-configured specialisation of the generic Requirements Gathering workflow for the C++ → C# migration context. Supplies all workflow parameters automatically; the caller only needs to provide `CppClass` and `CsProject`.

## Input (supplied by caller)

```
CppClass:   <C++ class name, e.g. InteiroLovelace>
CsProject:  <Target C# project name, e.g. Lovelace.Integer>
```

## Bindings

Follow `workflow-requirements-gathering` using the parameter bindings below.

**AnalysisSource**
> Run `skill-impl-completeness` with the provided `CppClass` and `CsProject`. Derive plain-English test descriptions from the C++ `.cpp` implementation in `Legacy/`.

**MandatoryItems**

*Mandatory constructors* — every numerical class must expose:

| Constructor | Signature | Tag |
|---|---|---|
| String constructor | `ctor(string value)` | `[mandatory — commodity parsing]` |
| ReadOnlySpan constructor | `ctor(ReadOnlySpan<char> value)` | `[mandatory — commodity parsing]` |

*Mandatory operator overloads* — every numerical class must implement the following via the corresponding `System.Numerics` interfaces:

| Operator | Interface | Tag |
|---|---|---|
| `operator+` (binary) | `IAdditionOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator-` (binary) | `ISubtractionOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator*` | `IMultiplyOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator/` | `IDivisionOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator%` | `IModulusOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator+` (unary) | `IUnaryPlusOperators<T,T>` | `[mandatory — arithmetic]` |
| `operator-` (unary) | `IUnaryNegationOperators<T,T>` | `[mandatory — arithmetic]` |
| `operator++` | `IIncrementOperators<T>` | `[mandatory — arithmetic]` |
| `operator--` | `IDecrementOperators<T>` | `[mandatory — arithmetic]` |
| `operator==`, `operator!=` | `IEqualityOperators<T,T,bool>` | `[mandatory — comparison]` |
| `operator<`, `operator>`, `operator<=`, `operator>=` | `IComparisonOperators<T,T,bool>` | `[mandatory — comparison]` |

> **Note**: Omit `operator%` and unary `operator-` for `Natural` (unsigned type). Include all operators for `Integer` and `Real`.

**PlanType**
> `"migration requirements"`

**OutputPath**
> `.github/requirements/<CsProject>.md`

**OutputTitle**
> `# Requirements: \`<CppClass>\` → \`<CsProject>\``

**ClosingMessage**
> "Requirements gathering complete. Output saved to `.github/requirements/<CsProject>.md`. Run `workflow-iterative-implementation` and supply one checklist item at a time to implement and test it end-to-end."

````
