using System.Runtime.InteropServices;

namespace wit_strings;

using System;
using System.Diagnostics;


public class StringsWorldImpl
{
    //TODO move to generated code
    [UnmanagedCallersOnly(EntryPoint = "test-imports")]
    public static void TestImportsExport()
    {
        TestImports();
    }

    public static void TestImports()
    {
    }

    public static string ReturnEmpty()
    {
        return "";
    }

    public static string Roundtrip(string t)
    {
        return t;
    }
}

// public class TestImpl : ITest
// {

// }