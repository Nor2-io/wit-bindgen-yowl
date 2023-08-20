namespace wit_numbers;

using wit_numbers.Wit.exports.test.numbers.Test;

using System;
using System.Diagnostics;

public static class NumbersImpl : NumbersWorld
{
    static void TestImports()
    {
        Debug.Assert(TestImpl.RoundtripU8(1) == 1);
        Debug.Assert(TestImpl.RoundtripU8(0) == 0);
        Debug.Assert(TestImpl.RoundtripU8(Byte.MaxValue) == Byte.MaxValue);

        Debug.Assert(TestImpl.RoundtripS8(1) == 1);
        Debug.Assert(TestImpl.RoundtripS8(SByte.MinValue) == SByte.MinValue);
        Debug.Assert(TestImpl.RoundtripS8(SByte.MaxValue) == SByte.MaxValue);

        Debug.Assert(TestImpl.RoundtripU16(1) == 1);
        Debug.Assert(TestImpl.RoundtripU16(0) == 0);
        Debug.Assert(TestImpl.RoundtripU16(UInt16.MaxValue) == UInt16.MaxValue);

        Debug.Assert(TestImpl.RoundtripS16(1) == 1);
        Debug.Assert(TestImpl.RoundtripS16(Int16.MinValue) == Int16.MinValue);
        Debug.Assert(TestImpl.RoundtripS16(Int16.MaxValue) == Int16.MaxValue);

        Debug.Assert(TestImpl.RoundtripU32(1) == 1);
        Debug.Assert(TestImpl.RoundtripU32(0) == 0);
        Debug.Assert(TestImpl.RoundtripU32(UInt32.MaxValue) == UInt32.MaxValue);

        Debug.Assert(TestImpl.RoundtripS32(1) == 1);
        Debug.Assert(TestImpl.RoundtripS32(Int32.MinValue) == Int32.MinValue);
        Debug.Assert(TestImpl.RoundtripS32(Int32.MaxValue) == Int32.MaxValue);

        Debug.Assert(TestImpl.RoundtripU64(1) == 1);
        Debug.Assert(TestImpl.RoundtripU64(0) == 0);
        Debug.Assert(TestImpl.RoundtripU64(UInt64.MaxValue) == UInt64.MaxValue);

        Debug.Assert(TestImpl.RoundtripS64(1) == 1);
        Debug.Assert(TestImpl.RoundtripS64(Int64.MinValue) == Int64.MinValue);
        Debug.Assert(TestImpl.RoundtripS64(Int64.MaxValue) == Int64.MaxValue);

        Debug.Assert(TestImpl.RoundtripFloat32(1.0) == 1.0);
        Debug.Assert(TestImpl.RoundtripFloat32(Single.PositiveInfinity) == Single.PositiveInfinity);
        Debug.Assert(TestImpl.RoundtripFloat32(Single.NegativeInfinity) == Single.NegativeInfinity);
        Debug.Assert(TestImpl.RoundtripFloat32(Single.NaN) == Single.NaN);

        Debug.Assert(TestImpl.RoundtripFloat64(1.0) == 1.0);
        Debug.Assert(TestImpl.RoundtripFloat64(Double.PositiveInfinity) == Double.PositiveInfinity);
        Debug.Assert(TestImpl.RoundtripFloat64(Double.NegativeInfinity) == Double.NegativeInfinity);
        Debug.Assert(TestImpl.RoundtripFloat64(Double.NaN) == Double.NaN);

        Debug.Assert(TestImpl.RoundtripChar(Char.ConvertToUtf32('a')) == Char.ConvertToUtf32('a'));
        Debug.Assert(TestImpl.RoundtripChar(Char.ConvertToUtf32(' ')) == Char.ConvertToUtf32(' '));
        Debug.Assert(TestImpl.RoundtripChar(Char.ConvertToUtf32("🚩"u8)) == Char.ConvertToUtf32("🚩"u8));

        TestImpl.SetScalar(2);
        Debug.Assert(TestImpl.GetScalar() == 2);
        TestImpl.SetScalar(4);
        Debug.Assert(TestImpl.GetScalar() == 4);
    }
}

public static class TestImpl : Test
{
    static uint SCALAR = 0;

    static int RoundtripU8(int p0)
    {
        return p0;
    }

    static int RoundtripS8(int p0)
    {
        return p0;
    }

    static int RoundtripU16(int p0)
    {
        return p0;
    }

    static int RoundtripS16(int p0)
    {
        return p0;
    }

    static int RoundtripU32(int p0)
    {
        return p0;
    }

    static int RoundtripS32(int p0)
    {
        return p0;
    }

    static long RoundtripU64(long p0)
    {
        return p0;
    }

    static long RoundtripS64(long p0)
    {
        return p0;
    }

    static float RoundtripFloat32(float p0)
    {
        return p0;
    }

    static double RoundtripFloat64(double p0)
    {
        return p0;
    }

    static int RoundtripChar(int p0)
    {
        return p0;
    }

    static void SetScalar(int p0)
    {
        return SCALAR = p0;
    }

    static int GetScalar()
    {
        return SCALAR;
    }
}