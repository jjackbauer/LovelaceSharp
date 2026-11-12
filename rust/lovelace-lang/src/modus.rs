//! The additive Modus plugin seam: registration, discoverable descriptors, call-site arity
//! enforcement, the typed payload channel, field-kernel dispatch and load/lifecycle refusals.
//!
//! The reference's `ModusHost` lives in `Lovelace.Suite` beside the interpreter because that host
//! owns the builtin registry (`Interpreter.Functions`). The Rust language crate is the equivalent
//! owner: `Engine` already owns the function/variable/assumption state and routes every call
//! through `Engine::call`, so the plugin registry belongs here rather than in the CLI runner. The
//! runner only parses argv and prints an envelope; it owns no registry state.
//!
//! This module is additive: a default `Engine` keeps every builtin it had, and a host that wants
//! the reference's opt-in model can construct `Engine::new_bare()`, load plugins explicitly and
//! watch the registry advertise exactly the builtins those plugins name.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use lovelace_array::NdArray;
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::Expr as Sym;
use lovelace_symbolics::assumptions::AssumptionSet;
use lovelace_symbolics::parameter_domain::{
    ParameterDomain, UndeclaredParameterDomain, parameter_domain_of, parameter_domain_of_ordinal,
};

use crate::context::Context;
use crate::registry;
use crate::{ErrorSignature, LangError, Value};

/// Whole-array elementwise operations a kernel can implement (the reference's `ArrayOp`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayOp {
    /// Elementwise addition.
    Add,
    /// Elementwise subtraction.
    Subtract,
    /// Elementwise multiplication.
    Multiply,
    /// Elementwise division.
    Divide,
}

/// The exact field identity the core injects into a kernel dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// `RealField` (the active precision scope applies).
    Real,
    /// `IntegerField`.
    Integer,
    /// `NaturalField`.
    Natural,
}

/// A pluggable elementwise kernel for one exact scalar type.
///
/// Returning `false` declines the request so dispatch falls back to the reference backend,
/// exactly as the C# `IFieldKernel<T>.TryElementwise` contract says.
pub trait FieldKernel<T>: Send + Sync {
    /// Tries to answer one elementwise operation, writing every result slot on success.
    fn try_elementwise(
        &self,
        op: ArrayOp,
        left: &[T],
        right: &[T],
        result: &mut [T],
        field: Field,
    ) -> bool;
}

/// A scalar type a kernel can be registered for.
///
/// Only the three exact numeric types have a field identity; any other `T` reports an unsupported
/// type and dispatch falls back without consulting a kernel (the C# `FieldFor<T>() == null` path).
pub trait KernelScalar: 'static {
    /// The field identity the core injects, or `None` when `T` has no exact field.
    fn field() -> Option<Field>;
}

impl KernelScalar for Real {
    fn field() -> Option<Field> {
        Some(Field::Real)
    }
}

impl KernelScalar for Int {
    fn field() -> Option<Field> {
        Some(Field::Integer)
    }
}

impl KernelScalar for Nat {
    fn field() -> Option<Field> {
        Some(Field::Natural)
    }
}

impl KernelScalar for i32 {
    fn field() -> Option<Field> {
        None
    }
}

impl KernelScalar for i64 {
    fn field() -> Option<Field> {
        None
    }
}

impl KernelScalar for u64 {
    fn field() -> Option<Field> {
        None
    }
}

/// A record payload's borrowed view: type name plus named fields.
pub type RecordView<'a> = (&'a str, &'a [(String, Payload)]);

/// The callable a plugin builtin stores.
type PluginCallable = dyn Fn(&PluginArgs) -> Result<Payload, PluginError> + Send + Sync;

/// A boxed plugin builtin implementation.
type PluginImplementation = Box<PluginCallable>;

/// A flat payload array plus its row-major shape.
#[derive(Clone, Debug, PartialEq)]
pub struct PayloadArray {
    /// The elements in row-major order.
    pub elements: Vec<Payload>,
    /// The published shape (a rank-1 vector is `[n]`).
    pub shape: Vec<usize>,
}

impl PayloadArray {
    /// A rank-1 array of the given elements.
    pub fn new(elements: Vec<Payload>) -> PayloadArray {
        let shape = vec![elements.len()];
        PayloadArray { elements, shape }
    }

    /// An array with an explicit shape.
    pub fn with_shape(elements: Vec<Payload>, shape: Vec<usize>) -> PayloadArray {
        PayloadArray { elements, shape }
    }
}

/// The plugin-facing payload vocabulary (the reference's `object?` channel).
///
/// A plugin never sees `Value`: the core unwraps each argument into this vocabulary before the
/// call and wraps the returned payload back. A `Record` keeps its type name, an `Enum` keeps both
/// components, and an `Array` keeps its shape, so the seam cannot silently degrade either kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Payload {
    /// A natural number.
    Natural(Nat),
    /// A signed integer.
    Integer(Int),
    /// A real number.
    Real(Real),
    /// A complex number (real, imaginary).
    Complex(Real, Real),
    /// A symbolic expression.
    Symbolic(Sym),
    /// A boolean.
    Boolean(bool),
    /// Text.
    Text(String),
    /// A record: its type name and named fields.
    Record(String, Vec<(String, Payload)>),
    /// A domain such as real or complex.
    Domain(String),
    /// A declared enumeration variant: (type name, member name).
    Enum(String, String),
    /// A vector or N-D array with its shape.
    Array(PayloadArray),
    /// A user-defined function value.
    Function(String),
    /// An absent structured field (the reference's `Value.Void`, carried as `null`).
    Void,
}

impl Payload {
    /// The kind name published in typed renderings and shape refusals.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Payload::Natural(_) => "Natural",
            Payload::Integer(_) => "Integer",
            Payload::Real(_) => "Real",
            Payload::Complex(_, _) => "Complex",
            Payload::Symbolic(_) => "Symbolic",
            Payload::Boolean(_) => "Boolean",
            Payload::Text(_) => "Text",
            Payload::Record(_, _) => "Record",
            Payload::Domain(_) => "Domain",
            Payload::Enum(_, _) => "Enum",
            Payload::Array(array) => {
                if array.shape.len() == 1 {
                    "Vector"
                } else {
                    "Array"
                }
            }
            Payload::Function(_) => "Function",
            Payload::Void => "Void",
        }
    }

    /// Unwraps a language value onto the plugin channel.
    pub fn from_value(value: &Value) -> Payload {
        match value {
            Value::Natural(n) => Payload::Natural(n.clone()),
            Value::Integer(i) => Payload::Integer(i.clone()),
            Value::Real(r) | Value::RealClamped(r, _, _) => Payload::Real(r.clone()),
            Value::Complex(re, im) => Payload::Complex(re.clone(), im.clone()),
            Value::Symbolic(e) => Payload::Symbolic(e.clone()),
            Value::Boolean(b) => Payload::Boolean(*b),
            Value::Text(t) => Payload::Text(t.clone()),
            Value::Record(name, fields) => Payload::Record(
                name.clone(),
                fields
                    .iter()
                    .map(|(field, value)| (field.clone(), Payload::from_value(value)))
                    .collect(),
            ),
            Value::Domain(name) => Payload::Domain(name.clone()),
            Value::Enum(type_name, member) => Payload::Enum(type_name.clone(), member.clone()),
            Value::Vector(items) => Payload::Array(PayloadArray::new(
                items.iter().map(Payload::from_value).collect(),
            )),
            Value::Array(array) => Payload::Array(PayloadArray::with_shape(
                array.iter().map(Payload::from_value).collect(),
                array.shape().to_vec(),
            )),
            Value::Function(name) => Payload::Function(name.clone()),
            Value::Void => Payload::Void,
        }
    }

    /// Wraps a payload back into a language value.
    pub fn into_value(self) -> Result<Value, LangError> {
        Ok(match self {
            Payload::Natural(n) => Value::Natural(n),
            Payload::Integer(i) => Value::Integer(i),
            Payload::Real(r) => Value::Real(r),
            Payload::Complex(re, im) => Value::Complex(re, im),
            Payload::Symbolic(e) => Value::Symbolic(e),
            Payload::Boolean(b) => Value::Boolean(b),
            Payload::Text(t) => Value::Text(t),
            Payload::Record(name, fields) => Value::Record(
                name,
                fields
                    .into_iter()
                    .map(|(field, value)| Ok((field, value.into_value()?)))
                    .collect::<Result<_, LangError>>()?,
            ),
            Payload::Domain(name) => Value::Domain(name),
            Payload::Enum(type_name, member) => Value::Enum(type_name, member),
            Payload::Array(array) => {
                let values = array
                    .elements
                    .into_iter()
                    .map(Payload::into_value)
                    .collect::<Result<Vec<_>, LangError>>()?;
                if array.shape.len() <= 1 {
                    Value::Vector(values)
                } else {
                    NdArray::from_vec(array.shape, values)
                        .map(Value::Array)
                        .map_err(|error| {
                            LangError::coded(
                                format!("A plugin returned an invalid array shape: {error}."),
                                0,
                                "InternalError",
                                "InternalInvariantFailure",
                            )
                        })?
                }
            }
            Payload::Function(name) => Value::Function(name),
            Payload::Void => Value::Void,
        })
    }

    /// Wraps a declared parameter domain as the `Domain` payload of its projected kernel domain.
    ///
    /// This is the Modus channel's form of the reference's `SymbolicsPlugin.ParameterDomainOf`:
    /// both declared arms project to the integer domain, which is what a plugin receives for a
    /// solution family's `parameter_domain`. The raw ordinal form is
    /// [`parameter_domain_payload_of_ordinal`], which refuses an undeclared value before a payload
    /// exists.
    pub fn from_parameter_domain(domain: ParameterDomain) -> Payload {
        Payload::Domain(parameter_domain_of(domain).name().to_string())
    }

    fn mismatch(&self, expected: &str) -> PluginError {
        PluginError::shape(expected, self.kind_name())
    }

    /// Reads an array payload, or reports the shape refusal.
    pub fn as_array(&self) -> Result<&PayloadArray, PluginError> {
        match self {
            Payload::Array(array) => Ok(array),
            other => Err(other.mismatch("an array or vector")),
        }
    }

    /// Reads a natural payload.
    pub fn as_natural(&self) -> Result<&Nat, PluginError> {
        match self {
            Payload::Natural(n) => Ok(n),
            other => Err(other.mismatch("a natural number")),
        }
    }

    /// Reads an integer payload.
    pub fn as_integer(&self) -> Result<&Int, PluginError> {
        match self {
            Payload::Integer(i) => Ok(i),
            other => Err(other.mismatch("an integer")),
        }
    }

    /// Reads a real payload.
    pub fn as_real(&self) -> Result<&Real, PluginError> {
        match self {
            Payload::Real(r) => Ok(r),
            other => Err(other.mismatch("a real number")),
        }
    }

    /// Reads a boolean payload.
    pub fn as_boolean(&self) -> Result<bool, PluginError> {
        match self {
            Payload::Boolean(b) => Ok(*b),
            other => Err(other.mismatch("a boolean")),
        }
    }

    /// Reads a text payload as a value-typed coercion (shape refusal on a non-text payload).
    pub fn as_text(&self) -> Result<&str, PluginError> {
        match self {
            Payload::Text(text) => Ok(text),
            other => Err(other.mismatch("text")),
        }
    }

    /// The hard cast a body may perform on its own payload: a payload of another kind is the
    /// cast refusal the call-site guard attributes to the argument the body was reading.
    pub fn cast_text(&self) -> Result<&str, PluginError> {
        match self {
            Payload::Text(text) => Ok(text),
            other => Err(PluginError::cast(other.kind_name())),
        }
    }

    /// Reads a record payload: its type name and fields.
    pub fn as_record(&self) -> Result<RecordView<'_>, PluginError> {
        match self {
            Payload::Record(name, fields) => Ok((name, fields)),
            other => Err(other.mismatch("a record")),
        }
    }

    /// Reads an enum payload: its type name and member name.
    pub fn as_enum(&self) -> Result<(&str, &str), PluginError> {
        match self {
            Payload::Enum(type_name, member) => Ok((type_name, member)),
            other => Err(other.mismatch("an enum")),
        }
    }

    /// Reads a symbolic payload.
    pub fn as_symbolic(&self) -> Result<&Sym, PluginError> {
        match self {
            Payload::Symbolic(e) => Ok(e),
            other => Err(other.mismatch("a symbolic expression")),
        }
    }

    /// Reads a domain payload.
    pub fn as_domain(&self) -> Result<&str, PluginError> {
        match self {
            Payload::Domain(name) => Ok(name),
            other => Err(other.mismatch("a domain")),
        }
    }
}

/// The Modus-channel projection of a raw parameter-domain ordinal.
///
/// An undeclared ordinal — the reference's `(ParameterDomain)99` — is refused with the reference
/// message, so a plugin can never receive a `Domain` payload the kernel lattice does not declare.
pub fn parameter_domain_payload_of_ordinal(
    ordinal: i32,
) -> Result<Payload, UndeclaredParameterDomain> {
    Ok(Payload::Domain(
        parameter_domain_of_ordinal(ordinal)?.name().to_string(),
    ))
}

/// The typed return channel: the reference's `ScalarResult`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarResult {
    /// The wrapped payload.
    pub payload: Payload,
}

impl ScalarResult {
    /// Wraps an arbitrary payload from the documented vocabulary.
    pub fn from(payload: Payload) -> ScalarResult {
        ScalarResult { payload }
    }

    /// Wraps a boolean result.
    pub fn from_boolean(value: bool) -> ScalarResult {
        ScalarResult::from(Payload::Boolean(value))
    }

    /// Wraps a text result.
    pub fn from_text(text: impl Into<String>) -> ScalarResult {
        ScalarResult::from(Payload::Text(text.into()))
    }

    /// Wraps a typed array result.
    pub fn from_array(array: PayloadArray) -> ScalarResult {
        ScalarResult::from(Payload::Array(array))
    }
}

/// A builtin's discoverability metadata (the reference's `BuiltinDescriptor`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltinDescriptor {
    /// The registered name.
    pub name: String,
    /// The declared parameter names.
    pub parameters: Vec<String>,
    /// The published category.
    pub category: String,
    /// The one-line summary.
    pub summary: String,
    /// The declared return kind.
    pub return_kind: String,
    /// The declared lower bound; `None` means exactly the declared parameter count.
    pub min_arity: Option<usize>,
    /// Whether the last declared parameter may repeat.
    pub variadic: bool,
}

impl BuiltinDescriptor {
    /// A descriptor whose declared parameter list is the exact arity.
    pub fn new(
        name: &str,
        parameters: &[&str],
        category: &str,
        summary: &str,
        return_kind: &str,
    ) -> BuiltinDescriptor {
        BuiltinDescriptor {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| (*p).to_string()).collect(),
            category: category.to_string(),
            summary: summary.to_string(),
            return_kind: return_kind.to_string(),
            min_arity: None,
            variadic: false,
        }
    }

    /// Declares a lower bound below the parameter count (an optional trailing parameter).
    pub fn with_min_arity(mut self, min_arity: usize) -> BuiltinDescriptor {
        self.min_arity = Some(min_arity);
        self
    }

    /// Declares the last parameter as repeating (removes the upper bound).
    pub fn with_variadic(mut self, variadic: bool) -> BuiltinDescriptor {
        self.variadic = variadic;
        self
    }
}

/// A plugin-channel failure. The call-site guard turns an attributed shape/cast failure into the
/// documented argument error; an unattributed one is an engine-internal coercion failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginError {
    /// A shape coercion failed: what was required and what arrived.
    Shape {
        /// What the accessor required.
        expected: String,
        /// The kind that arrived.
        arrived: String,
        /// The 0-based argument the value came from, when it came from an argument.
        position: Option<usize>,
    },
    /// A hard payload cast failed.
    Cast {
        /// The kind that arrived.
        arrived: String,
        /// The 0-based argument the value came from, when it came from an argument.
        position: Option<usize>,
    },
    /// A domain refusal raised by the plugin body.
    Domain(String),
    /// An engine-internal failure raised by the plugin body.
    Internal(String),
}

impl PluginError {
    /// A shape refusal.
    pub fn shape(expected: impl Into<String>, arrived: impl Into<String>) -> PluginError {
        PluginError::Shape {
            expected: expected.into(),
            arrived: arrived.into(),
            position: None,
        }
    }

    /// A hard-cast refusal.
    pub fn cast(arrived: impl Into<String>) -> PluginError {
        PluginError::Cast {
            arrived: arrived.into(),
            position: None,
        }
    }

    /// A domain refusal.
    pub fn domain(message: impl Into<String>) -> PluginError {
        PluginError::Domain(message.into())
    }

    /// An internal failure.
    pub fn internal(message: impl Into<String>) -> PluginError {
        PluginError::Internal(message.into())
    }

    fn at(mut self, position: usize) -> PluginError {
        match &mut self {
            PluginError::Shape { position: slot, .. }
            | PluginError::Cast { position: slot, .. } => {
                *slot = Some(position);
            }
            PluginError::Domain(_) | PluginError::Internal(_) => {}
        }
        self
    }
}

/// One argument handle: the payload plus the 0-based position it arrived at.
///
/// Accessors attribute a failed coercion to this position, exactly as the reference's
/// `ValueShapeException` carries the offending `Value` and the call-site guard maps it back to an
/// argument index.
pub struct Arg<'a> {
    payload: &'a Payload,
    position: usize,
}

impl<'a> Arg<'a> {
    /// The raw payload.
    pub fn payload(&self) -> &'a Payload {
        self.payload
    }

    /// The 0-based argument position.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Reads an array payload, attributed to this argument.
    pub fn as_array(&self) -> Result<&'a PayloadArray, PluginError> {
        self.payload.as_array().map_err(|e| e.at(self.position))
    }

    /// Reads a natural payload, attributed to this argument.
    pub fn as_natural(&self) -> Result<&'a Nat, PluginError> {
        self.payload.as_natural().map_err(|e| e.at(self.position))
    }

    /// Reads an integer payload, attributed to this argument.
    pub fn as_integer(&self) -> Result<&'a Int, PluginError> {
        self.payload.as_integer().map_err(|e| e.at(self.position))
    }

    /// Reads a real payload, attributed to this argument.
    pub fn as_real(&self) -> Result<&'a Real, PluginError> {
        self.payload.as_real().map_err(|e| e.at(self.position))
    }

    /// Reads a boolean payload, attributed to this argument.
    pub fn as_boolean(&self) -> Result<bool, PluginError> {
        self.payload.as_boolean().map_err(|e| e.at(self.position))
    }

    /// Reads a text payload, attributed to this argument.
    pub fn as_text(&self) -> Result<&'a str, PluginError> {
        self.payload.as_text().map_err(|e| e.at(self.position))
    }

    /// Performs the body's own hard text cast, attributed to this argument.
    pub fn cast_text(&self) -> Result<&'a str, PluginError> {
        self.payload.cast_text().map_err(|e| e.at(self.position))
    }

    /// Reads a record payload, attributed to this argument.
    pub fn as_record(&self) -> Result<RecordView<'a>, PluginError> {
        self.payload.as_record().map_err(|e| e.at(self.position))
    }

    /// Reads an enum payload, attributed to this argument.
    pub fn as_enum(&self) -> Result<(&'a str, &'a str), PluginError> {
        self.payload.as_enum().map_err(|e| e.at(self.position))
    }

    /// Reads a symbolic payload, attributed to this argument.
    pub fn as_symbolic(&self) -> Result<&'a Sym, PluginError> {
        self.payload.as_symbolic().map_err(|e| e.at(self.position))
    }
}

/// The argument list a plugin builtin body receives.
pub struct PluginArgs {
    args: Vec<Payload>,
}

impl PluginArgs {
    /// The number of arguments the call supplied.
    pub fn len(&self) -> usize {
        self.args.len()
    }

    /// True when the call supplied no arguments.
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    /// The handle for the 0-based argument, when it exists.
    pub fn get(&self, index: usize) -> Option<Arg<'_>> {
        self.args.get(index).map(|payload| Arg {
            payload,
            position: index,
        })
    }

    /// The raw payloads in argument order.
    pub fn iter(&self) -> impl Iterator<Item = &Payload> {
        self.args.iter()
    }
}

/// The registration surface a Modus plugin uses (the reference's `IModusContext`).
///
/// The host itself is the context: one mutable borrow is enough to register a builtin, a kernel,
/// an assumption context or a descriptor, and the host stamps every registration with the plugin
/// currently loading, exactly as `ModusHost.StampPlugin` does.
pub struct ModusHost {
    builtins: HashMap<String, PluginBuiltin>,
    kernels: Vec<Box<dyn AnyKernel>>,
    loaded_plugins: Vec<String>,
    loading_plugin: Option<String>,
    assumption_context: Option<Arc<Context>>,
}

impl Default for ModusHost {
    fn default() -> Self {
        ModusHost::new()
    }
}

impl ModusHost {
    /// Creates an empty plugin host.
    pub fn new() -> ModusHost {
        ModusHost {
            builtins: HashMap::new(),
            kernels: Vec::new(),
            loaded_plugins: Vec::new(),
            loading_plugin: None,
            assumption_context: None,
        }
    }

    /// Loads one plugin, refusing a second load of the same plugin name.
    ///
    /// The name is recorded BEFORE `register` runs, matching the reference's type set: a plugin
    /// whose registration fails stays marked loaded.
    pub fn load(&mut self, plugin: &mut dyn ModusPlugin) -> Result<(), LangError> {
        let name = plugin.name().to_string();
        if self.loaded_plugins.iter().any(|loaded| loaded == &name) {
            return Err(invalid_operation(format!(
                "Plugin '{name}' is already loaded."
            )));
        }
        self.loaded_plugins.push(name.clone());
        let previous = self.loading_plugin.replace(name);
        let result = plugin.register(self);
        self.loading_plugin = previous;
        result
    }

    /// True when the named plugin was loaded into this host.
    pub fn is_plugin_loaded(&self, name: &str) -> bool {
        self.loaded_plugins.iter().any(|loaded| loaded == name)
    }

    /// The plugin that registered this builtin, when it arrived through `load`.
    pub fn plugin_name_of(&self, name: &str) -> Option<&str> {
        self.builtins.get(name).and_then(|b| b.plugin.as_deref())
    }

    /// The descriptor registered with this builtin, when it carried one.
    pub fn descriptor_of(&self, name: &str) -> Option<&BuiltinDescriptor> {
        self.builtins.get(name).and_then(|b| b.descriptor.as_ref())
    }

    /// True when this host owns a builtin under the name.
    pub fn contains(&self, name: &str) -> bool {
        self.builtins.contains_key(name)
    }

    /// The registered plugin-builtin names, in name order.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.builtins.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Registers a general builtin with the declared parameter list as its exact arity.
    pub fn register_builtin<F>(
        &mut self,
        name: &str,
        parameters: &[&str],
        implementation: F,
    ) -> Result<(), LangError>
    where
        F: Fn(&PluginArgs) -> Result<Payload, PluginError> + Send + Sync + 'static,
    {
        self.register_inner(
            name,
            parameters,
            None,
            false,
            None,
            Box::new(implementation),
        )
    }

    /// Registers a general builtin over the typed `ScalarResult` return channel.
    pub fn register_builtin_scalar<F>(
        &mut self,
        name: &str,
        parameters: &[&str],
        implementation: F,
    ) -> Result<(), LangError>
    where
        F: Fn(&PluginArgs) -> Result<ScalarResult, PluginError> + Send + Sync + 'static,
    {
        self.register_builtin(name, parameters, move |args| {
            implementation(args).map(|result| result.payload)
        })
    }

    /// Registers a builtin together with its discoverability metadata.
    pub fn register_builtin_descriptor<F>(
        &mut self,
        descriptor: BuiltinDescriptor,
        implementation: F,
    ) -> Result<(), LangError>
    where
        F: Fn(&PluginArgs) -> Result<Payload, PluginError> + Send + Sync + 'static,
    {
        let name = descriptor.name.clone();
        let parameters: Vec<String> = descriptor.parameters.clone();
        let parameter_refs: Vec<&str> = parameters.iter().map(String::as_str).collect();
        let min_arity = descriptor.min_arity;
        let variadic = descriptor.variadic;
        self.register_inner(
            &name,
            &parameter_refs,
            min_arity,
            variadic,
            Some(descriptor),
            Box::new(implementation),
        )
    }

    /// Registers a descriptor-carrying builtin over the typed `ScalarResult` channel.
    pub fn register_builtin_descriptor_scalar<F>(
        &mut self,
        descriptor: BuiltinDescriptor,
        implementation: F,
    ) -> Result<(), LangError>
    where
        F: Fn(&PluginArgs) -> Result<ScalarResult, PluginError> + Send + Sync + 'static,
    {
        self.register_builtin_descriptor(descriptor, move |args| {
            implementation(args).map(|result| result.payload)
        })
    }

    /// Registers an array-to-array builtin (the reference's `RegisterArrayBuiltin`).
    pub fn register_array_builtin<F>(
        &mut self,
        name: &str,
        implementation: F,
    ) -> Result<(), LangError>
    where
        F: Fn(&PayloadArray) -> Result<PayloadArray, PluginError> + Send + Sync + 'static,
    {
        self.register_builtin(name, &["a"], move |args| {
            let argument = args
                .get(0)
                .ok_or_else(|| PluginError::internal("the array argument is missing"))?;
            let result = implementation(argument.as_array()?)?;
            Ok(Payload::Array(result))
        })
    }

    fn register_inner(
        &mut self,
        name: &str,
        parameters: &[&str],
        min_arity: Option<usize>,
        variadic: bool,
        descriptor: Option<BuiltinDescriptor>,
        implementation: PluginImplementation,
    ) -> Result<(), LangError> {
        if registry::find(name).is_some() || self.builtins.contains_key(name) {
            return Err(invalid_operation(format!(
                "A builtin named '{name}' is already registered; plugin builtins cannot shadow existing functions."
            )));
        }
        let plugin = self.loading_plugin.clone();
        self.builtins.insert(
            name.to_string(),
            PluginBuiltin {
                parameters: parameters.iter().map(|p| (*p).to_string()).collect(),
                min_arity,
                variadic,
                plugin,
                descriptor,
                implementation,
            },
        );
        Ok(())
    }

    /// Registers an exact-field elementwise kernel.
    pub fn register_kernel<T: KernelScalar>(&mut self, kernel: Box<dyn FieldKernel<T>>) {
        self.kernels.push(Box::new(KernelHolder { kernel }));
    }

    /// Fallible dispatch: tries every kernel registered for `T`, injecting the exact field.
    ///
    /// A type with no field identity (or a kernel that declines) reports `false`, which is the
    /// caller's signal to run the reference backend.
    pub fn try_dispatch<T: KernelScalar>(
        &self,
        op: ArrayOp,
        left: &[T],
        right: &[T],
        result: &mut [T],
    ) -> bool {
        let Some(field) = T::field() else {
            return false;
        };
        for kernel in &self.kernels {
            if let Some(holder) = kernel.as_any().downcast_ref::<KernelHolder<T>>() {
                if holder
                    .kernel
                    .try_elementwise(op, left, right, result, field)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Registers the plugin-instance assumption context the engine consults for caller scopes.
    pub fn register_assumption_context(&mut self, context: Arc<Context>) {
        self.assumption_context = Some(context);
    }

    /// The caller-installed assumption scope active on this thread, when a plugin registered one.
    pub fn active_assumption_scope(&self) -> Option<AssumptionSet> {
        self.assumption_context
            .as_ref()
            .filter(|context| context.assumption_scope_is_active())
            .map(|context| context.assumptions())
    }

    /// Calls a plugin builtin, when this host owns one under the name.
    pub fn call(
        &self,
        name: &str,
        args: &[Value],
        position: usize,
    ) -> Option<Result<Value, LangError>> {
        self.builtins
            .get(name)
            .map(|builtin| self.invoke(name, builtin, args, position))
    }

    fn invoke(
        &self,
        name: &str,
        builtin: &PluginBuiltin,
        args: &[Value],
        position: usize,
    ) -> Result<Value, LangError> {
        let actual = args.len();
        let (min, max) = builtin.declared_arity();
        if actual < min || (max != usize::MAX && actual > max) {
            return Err(arity_failure(name, builtin, actual, position));
        }
        let arguments = PluginArgs {
            args: args.iter().map(Payload::from_value).collect(),
        };
        match (builtin.implementation)(&arguments) {
            Ok(payload) => payload.into_value(),
            Err(error) => Err(plugin_failure(name, builtin, error, position)),
        }
    }
}

/// A Lovelace extension package: registers builtins, kernels or an assumption context.
pub trait ModusPlugin {
    /// The plugin's published name (the reference's `IModusPlugin.Name`).
    fn name(&self) -> &str;

    /// Registers the plugin's surface; the host stamps every registration with this name.
    fn register(&mut self, context: &mut ModusHost) -> Result<(), LangError>;
}

struct PluginBuiltin {
    parameters: Vec<String>,
    min_arity: Option<usize>,
    variadic: bool,
    plugin: Option<String>,
    descriptor: Option<BuiltinDescriptor>,
    implementation: PluginImplementation,
}

impl PluginBuiltin {
    fn declared_arity(&self) -> (usize, usize) {
        let declared = self.parameters.len();
        let min = self
            .min_arity
            .map(|min| min.min(declared))
            .unwrap_or(declared);
        let max = if self.variadic { usize::MAX } else { declared };
        (min, max)
    }

    fn signature(&self, name: &str, actual: usize) -> ErrorSignature {
        let (min, _) = self.declared_arity();
        ErrorSignature {
            function: name.to_string(),
            parameters: Some(self.parameters.clone()),
            expected_min: Some(min),
            expected_max: if self.variadic {
                None
            } else {
                Some(self.parameters.len())
            },
            variadic: Some(self.variadic),
            actual: Some(actual),
            ..ErrorSignature::default()
        }
    }
}

trait AnyKernel: Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

struct KernelHolder<T> {
    kernel: Box<dyn FieldKernel<T>>,
}

impl<T: 'static> AnyKernel for KernelHolder<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn invalid_operation(message: String) -> LangError {
    LangError::coded(message, 0, "InvalidOperation", "DomainError")
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "argument" } else { "arguments" }
}

fn arity_failure(name: &str, builtin: &PluginBuiltin, actual: usize, position: usize) -> LangError {
    let (min, max) = builtin.declared_arity();
    let expected = if builtin.variadic {
        format!("at least {} {}", min, plural(min))
    } else if min == max {
        format!("{} {}", min, plural(min))
    } else {
        format!("{} to {} arguments", min, max)
    };
    let mut signature = builtin.signature(name, actual);
    signature.actual = Some(actual);
    LangError::coded(
        format!("{name}(): expected {expected}; got {actual}."),
        position,
        "InvalidArgument",
        "TypeMismatch",
    )
    .with_signature(signature)
}

fn plugin_failure(
    name: &str,
    builtin: &PluginBuiltin,
    error: PluginError,
    position: usize,
) -> LangError {
    match error {
        PluginError::Shape {
            expected,
            arrived,
            position: Some(index),
        } => argument_shape_failure(name, builtin, &expected, &arrived, index, position),
        PluginError::Cast {
            arrived,
            position: Some(index),
        } => argument_shape_failure(
            name,
            builtin,
            "a value this builtin can use",
            &arrived,
            index,
            position,
        ),
        PluginError::Shape {
            expected,
            arrived,
            position: None,
        } => LangError::coded(
            format!(
                "{name}(): an engine-internal value failed coercion; expected {expected}, got {arrived}."
            ),
            position,
            "InternalError",
            "InternalInvariantFailure",
        ),
        PluginError::Cast {
            arrived,
            position: None,
        } => LangError::coded(
            format!("{name}(): an engine-internal value failed its payload cast; got {arrived}."),
            position,
            "InternalError",
            "InternalInvariantFailure",
        ),
        PluginError::Domain(message) => {
            LangError::coded(message, position, "InvalidOperation", "DomainError")
        }
        PluginError::Internal(message) => LangError::coded(
            message,
            position,
            "InternalError",
            "InternalInvariantFailure",
        ),
    }
}

fn argument_shape_failure(
    name: &str,
    builtin: &PluginBuiltin,
    expected: &str,
    arrived: &str,
    index: usize,
    position: usize,
) -> LangError {
    let argument = index + 1;
    let mut signature = builtin.signature(name, builtin.parameters.len());
    signature.position = Some(argument);
    signature.parameter = builtin.parameters.get(index).cloned();
    signature.expected_shape = Some(expected.to_string());
    signature.arrived_kind = Some(arrived.to_string());
    LangError::coded(
        format!("{name}(): argument {argument} must be {expected}; got {arrived}."),
        position,
        "InvalidArgument",
        "TypeMismatch",
    )
    .with_signature(signature)
}
