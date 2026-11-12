//! The runner's view of the plot surface, which now lives in `lovelace-lang::plot_surface`.
//!
//! Round 87 moved the implementation into the language crate so the runner's `plot()` builtin and
//! this surface are one renderer instead of two: a binary whose `plot()` emitted a 326-byte stub
//! was the whole divergence (#97/#98). Nothing is re-implemented here; the module re-exports the
//! language crate's surface so the runner keeps a single `crate::plot_surface` path.
//!
//! The unit tests below are the ported `PlotTests` rows and stay in this file because the census
//! rows in `parity/manifest.toml` name these test ids (`lovelace-run::plot_surface::tests::…`).
//! They exercise the re-exported implementation, so moving the code did not move the evidence.

#[allow(unused_imports)]
pub use lovelace_lang::plot_surface::*;

#[cfg(test)]
mod tests {
    use super::*;
    use lovelace_lang::Value;
    use lovelace_nat::Nat;
    use lovelace_real::Real;

    fn points(values: &[(f64, f64)]) -> PlotSeries {
        let mut series = PlotSeries::new();
        for (x, y) in values {
            series.push(PlotPoint::from_f64(*x, *y));
        }
        series
    }

    fn points_attribute(svg: &str) -> &str {
        let needle = "points=\"";
        let start = svg.find(needle).expect("the SVG carries a polyline") + needle.len();
        let tail = &svg[start..];
        &tail[..tail.find('"').expect("the attribute closes")]
    }

    #[test]
    fn plot_value_given_huge_natural_to_real_preserves_digits_exactly() {
        // PlotTests.PlotValue_GivenHugeNatural_ToRealPreservesDigitsExactly: a 401-digit
        // Natural must not round-trip through a double or a lossy string.
        let digits = format!("1{}", "0".repeat(400));
        let value = Value::Natural(Nat::from_decimal(&digits).expect("401 digits parse"));
        let real = PlotValue::to_real(&value).expect("a Natural promotes exactly");
        assert_eq!(real.to_string(), digits);
    }

    #[test]
    fn plot_value_given_periodic_real_expands_to_double() {
        // PlotTests.PlotValue_GivenPeriodicReal_ExpandsToDouble: Rl.Parse("0.(3)") to double is
        // 1/3 to three decimal places.
        let value = Value::Real(Real::from_str("0.(3)").expect("periodic syntax parses"));
        let value = PlotValue::to_double(&value).expect("a Real converts");
        assert!((value - 1.0 / 3.0).abs() < 0.001, "{value}");
    }

    #[test]
    fn plot_value_refuses_non_numeric_kinds_with_the_reference_message() {
        let error = PlotValue::to_real(&Value::Text("x".to_string())).expect_err("text refuses");
        assert_eq!(error.code, "InvalidOperation");
        assert_eq!(error.category, "DomainError");
        assert_eq!(
            error.message,
            "Cannot convert value of kind 'Text' to a number for plotting."
        );
    }

    #[test]
    fn svg_given_linear_interpolation_emits_polyline() {
        // PlotTests.SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline: the C# body builds a
        // three-point series with `Interpolation = PlotInterpolation.Linear`, then requires a
        // `<polyline` and no `<path`.
        let series = PlotSeries::with_interpolation(PlotInterpolation::Linear).with_points(
            [(1.0, 1.0), (2.0, 4.0), (3.0, 9.0)]
                .iter()
                .map(|(x, y)| PlotPoint::from_f64(*x, *y)),
        );
        let model = PlotModel::new().with_series(series);
        let svg = SvgPlotRenderer::new().render(&model);
        assert!(svg.contains("<polyline"), "{svg}");
        assert!(!svg.contains("<path"), "{svg}");
        // Linear keeps one coordinate per data point (the C# `AppendPolyline`); the CubicSpline
        // default densely samples the same three points, so this pins the row's branch.
        assert_eq!(points_attribute(&svg).split(' ').count(), 3, "{svg}");
    }

    #[test]
    fn svg_given_cubic_spline_passes_through_data_endpoints() {
        let data = [(1.0, 1.0), (2.0, 4.0), (3.0, 9.0), (4.0, 16.0)];
        let linear = PlotModel::new().with_series(
            PlotSeries::with_interpolation(PlotInterpolation::Linear)
                .with_points(data.iter().map(|(x, y)| PlotPoint::from_f64(*x, *y))),
        );
        let spline = PlotModel::new().with_series(
            PlotSeries::with_interpolation(PlotInterpolation::CubicSpline)
                .with_points(data.iter().map(|(x, y)| PlotPoint::from_f64(*x, *y))),
        );

        let linear_svg = SvgPlotRenderer::new().render(&linear);
        let spline_svg = SvgPlotRenderer::new().render(&spline);
        let linear_points: Vec<&str> = points_attribute(&linear_svg).split(' ').collect();
        let spline_points: Vec<&str> = points_attribute(&spline_svg).split(' ').collect();
        assert_eq!(linear_points.first(), spline_points.first());
        assert_eq!(linear_points.last(), spline_points.last());
    }

    #[test]
    fn svg_given_huge_close_x_values_keeps_points_distinct() {
        // PlotTests.SvgPlotRenderer_GivenHugeCloseXValues_KeepsPointsDistinct: 10^20 and
        // 10^20 + 1 differ by one unit in exact arithmetic, so they cannot collapse through a
        // double rounding at the range-normalisation step.
        let mut series = PlotSeries::new();
        series.push(PlotPoint::new(
            Real::from_str("100000000000000000000").expect("huge x1"),
            Real::zero(),
        ));
        series.push(PlotPoint::new(
            Real::from_str("100000000000000000001").expect("huge x2"),
            Real::from_i64(1),
        ));
        let model = PlotModel::new().with_series(series);
        let svg = SvgPlotRenderer::new().render(&model);

        assert!(!svg.contains("Infinity"), "{svg}");
        assert!(!svg.contains("NaN"), "{svg}");
        let coordinates: Vec<&str> = points_attribute(&svg).split(' ').collect();
        assert_eq!(coordinates.len(), 2, "{svg}");
        let x1: f64 = coordinates[0].split(',').next().unwrap().parse().unwrap();
        let x2: f64 = coordinates[1].split(',').next().unwrap().parse().unwrap();
        assert_ne!(x1, x2, "{svg}");
    }

    #[test]
    fn svg_given_three_points_uses_dense_spline_polyline() {
        let model = PlotModel::new().with_series(points(&[(1.0, 1.0), (2.0, 4.0), (3.0, 9.0)]));
        let svg = SvgPlotRenderer::new().render(&model);
        assert!(svg.contains("<polyline"), "{svg}");
        assert!(!svg.contains("<path"), "{svg}");
        assert!(
            points_attribute(&svg).split(' ').count() > 3,
            "expected a densely sampled spline: {svg}"
        );
    }

    #[test]
    fn renderer_options_size_the_document_and_change_the_palette() {
        let options = RenderOptions {
            width: 320.0,
            height: 200.0,
            series_colors: vec!["#000000".to_string()],
            ..RenderOptions::default()
        };
        let model = PlotModel::new().with_series(points(&[(1.0, 2.0), (2.0, 4.0)]));
        let svg = SvgPlotRenderer::with_options(options).render(&model);
        assert!(svg.contains("width=\"320\" height=\"200\""), "{svg}");
        assert!(svg.contains("stroke=\"#000000\""), "{svg}");
    }

    #[test]
    fn wire_serialisation_publishes_the_model_and_the_runner_plot_block() {
        let mut series = PlotSeries::with_interpolation(PlotInterpolation::Linear);
        series.push(PlotPoint::from_f64(1.0, 2.0));
        let model = PlotModel {
            title: Some("T".to_string()),
            series: vec![series],
        };
        assert_eq!(
            model.to_wire_json().to_string(),
            r#"{"series":[{"interpolation":"Linear","points":[{"x":"1","y":"2"}]}],"title":"T"}"#
        );

        let capture = PlotCapture::new("<svg/>".to_string(), Some("T".to_string()));
        assert_eq!(
            capture.to_wire_json("C:/plots/p.svg").to_string(),
            r#"{"path":"C:/plots/p.svg","svg":"<svg/>","title":"T"}"#
        );
    }
}
