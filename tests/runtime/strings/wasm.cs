using System;
using System.Diagnostics;
using wit_strings.Wit.imports.test.strings.Imports;

namespace wit_strings;

public class StringsWorldImpl : StringsWorld
{
    public static void TestImports()
    {
        Debug.Assert(ReturnEmpty() == "");
        Debug.Assert(Roundtrip("a") == "a");
        Debug.Assert(Roundtrip("🚀🚀🚀 𠈄𓀀") == "🚀🚀🚀 𠈄𓀀");

        ImportsInterop.TakeBasic("latin utf16");
        Debug.Assert(ImportsInterop.ReturnUnicode() == "🚀🚀🚀 𠈄𓀀");
    }

    public static string ReturnEmpty()
    {
        return "";
    }

    public static string Roundtrip(string s)
    {
        Console.WriteLine(s);
        return s;
    }
}