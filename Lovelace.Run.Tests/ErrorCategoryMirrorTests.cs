using Lovelace.Abstractions;

namespace Lovelace.Run.Tests;

/// <summary>
/// The error taxonomy is mirrored as a literal array in three envelope tests, so that a category a
/// test observes is proven to be a member of the DOCUMENTED set rather than merely a non-empty
/// string. A mirror is only worth that while it equals the enum it claims to mirror, so this is the
/// one place that fails when <see cref="ErrorCategory"/> gains a member and a mirror is not updated.
/// </summary>
public class ErrorCategoryMirrorTests
{
    [Fact]
    public void EveryMirroredTaxonomy_EqualsTheEnum()
    {
        string[] declared = Enum.GetNames<ErrorCategory>();

        Assert.Equal(declared, AllocationRefusalEnvelopeTests.ErrorCategories);
        Assert.Equal(declared, PlotDirectoryErrorTests.ErrorCategories);
        Assert.Equal(declared, PlotFileErrorTests.ErrorCategories);
    }
}
