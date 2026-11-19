using Snowflake.Data.Compatibility;

namespace Snowflake.Data.Tests.Compatibility;

[Trait("Category", "Unit")]
public sealed class EnumHelperTest
{
    private enum Empty
    {
    }

    private enum OneMember
    {
        Only,
    }

    private enum OutOfOrder
    {
        Five = 5,
        Zero = 0,
        Two = 2,
    }

    private enum Aliased
    {
        First = 1,
        AlsoFirst = 1,
        Second = 2,
    }

    private enum Signed : sbyte
    {
        Min = sbyte.MinValue,
        MinusOne = -1,
        Zero = 0,
        Max = sbyte.MaxValue,
    }

    private enum Wide : ulong
    {
        Zero = 0,
        Max = ulong.MaxValue,
    }

    [Flags]
    private enum Permissions
    {
        None = 0,
        Read = 1,
        Write = 2,
        ReadWrite = Read | Write,
        Execute = 4,
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EnumWithoutMembersHasNoValues()
    {
        Assert.Empty(EnumHelper.GetValues<Empty>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EnumWithOneMemberHasThatValue()
    {
        Assert.Equal([OneMember.Only], EnumHelper.GetValues<OneMember>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ValuesAreSortedByValueNotByDeclarationOrder()
    {
        Assert.Equal([OutOfOrder.Zero, OutOfOrder.Two, OutOfOrder.Five], EnumHelper.GetValues<OutOfOrder>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MembersSharingAValueAreEachReturned()
    {
        Assert.Equal([Aliased.First, Aliased.First, Aliased.Second], EnumHelper.GetValues<Aliased>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NegativeValuesAreSortedAfterNonNegativeOnes()
    {
        Assert.Equal(
            [Signed.Zero, Signed.Max, Signed.Min, Signed.MinusOne],
            EnumHelper.GetValues<Signed>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ValuesOfAnUnsignedLongEnumKeepTheFullRange()
    {
        Assert.Equal([Wide.Zero, Wide.Max], EnumHelper.GetValues<Wide>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FlagsEnumReturnsOnlyDeclaredMembers()
    {
        Assert.Equal(
            [Permissions.None, Permissions.Read, Permissions.Write, Permissions.ReadWrite, Permissions.Execute],
            EnumHelper.GetValues<Permissions>());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EachCallReturnsAnIndependentArray()
    {
        var first = EnumHelper.GetValues<OutOfOrder>();
        first[0] = OutOfOrder.Five;

        var second = EnumHelper.GetValues<OutOfOrder>();

        Assert.NotSame(first, second);
        Assert.Equal([OutOfOrder.Zero, OutOfOrder.Two, OutOfOrder.Five], second);
    }
}
