//! Round-30: the Suite StructuredRealExactness row the new fixed-width LReal tier unblocks.
//!
//! C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:196:
//!   LReal64 periodic = LReal64.Parse("0.(0588235294117647)");
//!   LReal64 product = periodic * LReal64.Parse("17");
//!   Assert.True(product != LReal64.One);
//!   StructuredValueDto sv = StructuredProjection.ToStructured(new Value(product.ToReal()));
//!   Assert.True(sv.Exact != true);
//! The fixed-width product is the truncated 0.999999999999999985, so its promotion must carry
//! inexact provenance instead of being laundered into an exact Real.

use lovelace_json::Json;
use lovelace_real::lreal::LReal64;

use crate::structured::project_value;
use crate::{Engine, Value};

#[test]
fn fixed_width_truncation_is_not_published_as_an_exact_real() {
    let periodic = LReal64::parse("0.(0588235294117647)").expect("periodic literal");
    let product = periodic
        .mul(&LReal64::parse("17").expect("seventeen"))
        .expect("the product fits the width");
    assert!(
        product != LReal64::one(),
        "the fixed-width product must be the truncated 0.999...985, not one: {product}"
    );

    let real = product.to_real();
    let projected = project_value(&Engine::new(), &Value::Real(real), None);
    assert!(
        !matches!(projected.get("exact"), Some(Json::Bool(true))),
        "the fixed-width truncation was published as an exact Real: {}",
        projected.to_string()
    );
}
