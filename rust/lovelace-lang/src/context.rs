//! A thread-scoped expression context mirroring the reference's `ExprContext`.
//!
//! The reference binds symbol and function identities to a context, and it lets a caller install
//! an immutable assumption set for the duration of a scope (`WithAssumptions`). There the scope is
//! flow-local through `AsyncLocal`; here it is thread-local, which is the equivalent a synchronous
//! Rust harness can observe: a scope is visible on the thread that opened it, invisible to every
//! other thread, restores the enclosing scope when the guard drops, and restores the base set when
//! the thread unwinds through the guard.
//!
//! The context also owns the two race-free interning tables the reference keeps on `ExprContext`
//! (`Symbol` and `Function`): ids are allocated under one lock, so concurrent creation can never
//! produce a duplicate id, and every id round-trips to its name.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use lovelace_symbolics::assumptions::AssumptionSet;
use lovelace_symbolics::{CanonicalParseError, Expr};

/// The next context id. Zero is reserved so a context id is never the default `u64`.
static NEXT_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The next scope id. Scope ids are unique across threads, so a guard can remove exactly the layer
/// it installed even when guards are dropped out of order.
static NEXT_SCOPE_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// The active scope stack per context on THIS thread. A context absent from the map has no
    /// scope on this thread and therefore reads its base set.
    static ACTIVE_SCOPES: RefCell<HashMap<u64, Vec<ActiveScope>>> = RefCell::new(HashMap::new());
}

/// One active scope layer: the unique guard id and the immutable set it installed.
#[derive(Clone, Debug)]
struct ActiveScope {
    id: u64,
    assumptions: AssumptionSet,
}

/// An interned symbol identity: a dense id and the canonical name it denotes.
///
/// Equality, hashing and ordering are by canonical name (ordinal), matching the reference's
/// `Symbol`, so identities from different contexts agree when their names agree. The id is the
/// context-local dense identity and is what `Context::symbol_by_id` round-trips.
#[derive(Clone, Debug)]
pub struct Symbol {
    id: usize,
    name: String,
}

impl Symbol {
    /// The dense context-local id.
    pub fn id(&self) -> usize {
        self.id
    }

    /// The canonical name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Symbol {}

impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// An interned function identity: a dense id and the canonical name it denotes.
///
/// Equality, hashing and ordering are by canonical name (ordinal), matching the reference's
/// `FunctionId`. The id is the context-local dense identity and is what
/// `Context::function_by_id` round-trips.
#[derive(Clone, Debug)]
pub struct FunctionId {
    id: usize,
    name: String,
}

impl FunctionId {
    /// The dense context-local id.
    pub fn id(&self) -> usize {
        self.id
    }

    /// The canonical name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl PartialEq for FunctionId {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for FunctionId {}

impl Hash for FunctionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialOrd for FunctionId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FunctionId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl fmt::Display for FunctionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// The name table behind one of the two interning surfaces.
///
/// Both maps live under the same mutex, so a concurrent intern either observes the complete
/// `(name, id)` pair or waits for the writer; a half-updated table is never observable.
struct IdTable {
    next: usize,
    ids: HashMap<String, usize>,
    names: HashMap<usize, String>,
}

impl IdTable {
    fn new() -> IdTable {
        IdTable {
            next: 0,
            ids: HashMap::new(),
            names: HashMap::new(),
        }
    }

    fn intern(&mut self, name: &str) -> usize {
        if let Some(id) = self.ids.get(name) {
            return *id;
        }
        let id = self.next;
        self.next += 1;
        self.ids.insert(name.to_string(), id);
        self.names.insert(id, name.to_string());
        id
    }

    fn name(&self, id: usize) -> Option<&str> {
        self.names.get(&id).map(String::as_str)
    }
}

/// A per-engine identity table and assumption scope: the Rust analogue of the reference's
/// `ExprContext`.
///
/// The context is `Send + Sync`: symbol and function interning is race-free, the base assumption
/// set is behind a lock, and a scope is stored in the opening thread's local map, so it can never
/// leak into another thread's view.
pub struct Context {
    id: u64,
    symbols: Mutex<IdTable>,
    functions: Mutex<IdTable>,
    base_assumptions: RwLock<AssumptionSet>,
}

impl Context {
    /// Creates an empty context.
    ///
    /// Unlike the reference constructor, no function registry is registered: the Rust language
    /// registry is global and nothing this module observes reads a per-context registry.
    pub fn new() -> Context {
        Context {
            id: NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed),
            symbols: Mutex::new(IdTable::new()),
            functions: Mutex::new(IdTable::new()),
            base_assumptions: RwLock::new(AssumptionSet::empty()),
        }
    }

    /// Interns a symbol by canonical name.
    ///
    /// The same name always returns the same id; different names never share an id, including
    /// when they are first interned concurrently on several threads.
    pub fn symbol(&self, name: &str) -> Symbol {
        let id = table_lock(&self.symbols).intern(name);
        Symbol {
            id,
            name: name.to_string(),
        }
    }

    /// Resolves a dense symbol id back to its identity, or `None` when the id was never allocated
    /// in this context.
    pub fn symbol_by_id(&self, id: usize) -> Option<Symbol> {
        table_lock(&self.symbols).name(id).map(|name| Symbol {
            id,
            name: name.to_string(),
        })
    }

    /// Interns a function identity by canonical name.
    ///
    /// The same name always returns the same id; different names never share an id, including
    /// when they are first interned concurrently on several threads.
    pub fn function(&self, name: &str) -> FunctionId {
        let id = table_lock(&self.functions).intern(name);
        FunctionId {
            id,
            name: name.to_string(),
        }
    }

    /// Resolves a dense function id back to its identity, or `None` when the id was never
    /// allocated in this context.
    pub fn function_by_id(&self, id: usize) -> Option<FunctionId> {
        table_lock(&self.functions).name(id).map(|name| FunctionId {
            id,
            name: name.to_string(),
        })
    }

    /// The active assumption set: the innermost scope installed on THIS thread, otherwise the
    /// context's base set.
    pub fn assumptions(&self) -> AssumptionSet {
        self.scoped_assumptions()
            .unwrap_or_else(|| base_read(&self.base_assumptions).clone())
    }

    /// Replaces the context's base assumption set.
    ///
    /// An active scope still overrides the base set for its own thread, exactly as the
    /// reference's setter updates the base while a flow-local scope keeps precedence.
    pub fn set_assumptions(&self, assumptions: AssumptionSet) {
        *base_write(&self.base_assumptions) = assumptions;
    }

    /// True while a scope installed by `Context::with_assumptions` is active on THIS thread.
    pub fn assumption_scope_is_active(&self) -> bool {
        ACTIVE_SCOPES.with(|scopes| {
            scopes
                .borrow()
                .get(&self.id)
                .is_some_and(|stack| !stack.is_empty())
        })
    }

    /// Installs `assumptions` for the lifetime of the returned guard.
    ///
    /// The scope is thread-local: another thread sees its own stack (or the base set), and a
    /// guard dropped on a different thread is impossible because `AssumptionScope` is not
    /// `Send`. Nested scopes restore the enclosing set when the inner guard drops, and the base
    /// set is never mutated by a scope. If the opening thread unwinds, the guard's `Drop` restores
    /// the enclosing set before the panic leaves the thread.
    pub fn with_assumptions(&self, assumptions: AssumptionSet) -> AssumptionScope<'_> {
        let scope_id = NEXT_SCOPE_ID.fetch_add(1, Ordering::Relaxed);
        ACTIVE_SCOPES.with(|scopes| {
            scopes
                .borrow_mut()
                .entry(self.id)
                .or_default()
                .push(ActiveScope {
                    id: scope_id,
                    assumptions,
                });
        });
        AssumptionScope {
            context: self,
            scope_id,
            _thread_bound: PhantomData,
        }
    }

    /// Parses the reference's canonical (typed S-expression) form, interning every symbol and
    /// function identity the expression mentions.
    ///
    /// This is the analogue of `Printing.CanonicalParse(text, ctx)`: the returned expression is
    /// canonical, and the context's two tables now own the identities it reads.
    pub fn canonical_parse(&self, text: &str) -> Result<Expr, CanonicalParseError> {
        let expression = lovelace_symbolics::canonical_parse(text)?;
        self.note_identities(&expression);
        Ok(expression)
    }

    /// Simplifies an expression under the context's ACTIVE assumption set.
    ///
    /// The scratch engine exists only to carry the scoped set into the existing simplifier, so
    /// simplification inside a scope is licensed by that scope's assumptions and nothing else.
    pub fn simplify(&self, expression: Expr) -> Expr {
        let mut engine = crate::Engine::new();
        engine.set_assumptions(self.assumptions());
        crate::builtins::simplify(&engine, expression)
    }

    /// Differentiates an expression with respect to `variable`.
    pub fn diff(&self, expression: &Expr, variable: &str) -> Expr {
        lovelace_symbolics::diff(expression, variable)
    }

    /// Lowers an expression to MathIR for the named parameters.
    pub fn lower(
        &self,
        expression: &Expr,
        parameters: &[String],
    ) -> Result<crate::mathir::IrProgram, crate::LangError> {
        crate::mathir::IrProgram::lower(expression, parameters)
    }

    /// The assumptions of the innermost scope on this thread, when one is active.
    fn scoped_assumptions(&self) -> Option<AssumptionSet> {
        ACTIVE_SCOPES.with(|scopes| {
            scopes
                .borrow()
                .get(&self.id)
                .and_then(|stack| stack.last())
                .map(|scope| scope.assumptions.clone())
        })
    }

    /// Interns every symbol and function name the canonical expression mentions.
    fn note_identities(&self, expression: &Expr) {
        match expression {
            Expr::Sym(name) => {
                self.symbol(name);
            }
            Expr::Fun(name, arguments) => {
                self.function(name);
                for argument in arguments {
                    self.note_identities(argument);
                }
            }
            Expr::Add(items) | Expr::Mul(items) => {
                for item in items {
                    self.note_identities(item);
                }
            }
            Expr::Pow(left, right) | Expr::Div(left, right) => {
                self.note_identities(left);
                self.note_identities(right);
            }
            Expr::Rel(_, left, right) => {
                self.note_identities(left);
                self.note_identities(right);
            }
            Expr::Num(_) | Expr::Real(_) | Expr::Complex(_, _) => {}
        }
    }
}

impl Default for Context {
    fn default() -> Context {
        Context::new()
    }
}

/// An RAII guard installed by `Context::with_assumptions`.
///
/// The guard is thread-bound (it is neither `Send` nor `Sync`): its scope was only ever visible
/// on the thread that opened it, so that is the only thread that may end it. Dropping the guard
/// restores the enclosing scope or, when it was the outermost scope, the context's base set.
/// `Drop` runs during ordinary block exit and during panic unwinding alike.
pub struct AssumptionScope<'a> {
    context: &'a Context,
    scope_id: u64,
    /// Raw-pointer marker: makes the guard thread-bound without `unsafe` and without storing a
    /// real pointer.
    _thread_bound: PhantomData<*const ()>,
}

impl Drop for AssumptionScope<'_> {
    fn drop(&mut self) {
        ACTIVE_SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            if let Some(stack) = scopes.get_mut(&self.context.id) {
                if let Some(index) = stack.iter().rposition(|scope| scope.id == self.scope_id) {
                    stack.remove(index);
                }
                if stack.is_empty() {
                    scopes.remove(&self.context.id);
                }
            }
        });
    }
}

/// Locks one interning table, recovering the table if a panic poisoned the mutex. The table is
/// always left complete by every operation that holds this lock, so recovery is sound.
fn table_lock(table: &Mutex<IdTable>) -> MutexGuard<'_, IdTable> {
    table.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Reads the base assumption set, recovering it if a panic poisoned the lock.
fn base_read(set: &RwLock<AssumptionSet>) -> RwLockReadGuard<'_, AssumptionSet> {
    set.read().unwrap_or_else(PoisonError::into_inner)
}

/// Writes the base assumption set, recovering it if a panic poisoned the lock.
fn base_write(set: &RwLock<AssumptionSet>) -> RwLockWriteGuard<'_, AssumptionSet> {
    set.write().unwrap_or_else(PoisonError::into_inner)
}
