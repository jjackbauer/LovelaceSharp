using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The rule → proof-obligation bridge (Cycle 2 item 23) must stay in step with the registry. This
/// is the invariant that gives the map teeth: adding a rewrite rule without stating its proof
/// obligation fails here, and an obligation for a rule that no longer exists fails too.
/// </summary>
public class RewriteProofBridgeTests
{
    [Fact]
    public void EveryShippedRule_HasExactlyOneProofObligation()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;

        var shipped = Simplify.ShippedRuleIds(ctx).OrderBy(id => id, StringComparer.Ordinal).ToArray();
        var stated = RewriteProofs.Obligations.Keys.OrderBy(id => id, StringComparer.Ordinal).ToArray();

        Assert.Equal(shipped, stated);
    }

    [Fact]
    public void EveryObligation_IsStatedAndHonestAboutItsStatus()
    {
        foreach (var (id, obligation) in RewriteProofs.Obligations)
        {
            Assert.Equal(id, obligation.RuleId);
            Assert.False(string.IsNullOrWhiteSpace(obligation.Statement),
                id + " has an empty proof statement");
            if (obligation.Status != ProofStatus.MachineChecked)
                Assert.Null(obligation.Theorem);   // no prover name without a checked proof
        }
    }
}
