using System;
using System.Diagnostics;
using System.Linq;
using Microsoft.CodeAnalysis;

namespace WitSourceGen
{
    [Generator]
    public class WitSourceGenerator : ISourceGenerator
    {
        public void Execute(GeneratorExecutionContext context)
        {

            // find anything that matches our files
            var witFiles = context.AdditionalFiles.Where(at => at.Path.EndsWith(".wit"));

            var output = context.GetMsBuildProperty("WitCompilerGeneratedFilesOutputPath", "Generated");
            var witBindgenPath = context.GetMsBuildProperty("WitBindgenPath", "");

            if (!string.IsNullOrWhiteSpace(witBindgenPath) && !witBindgenPath.EndsWith("\\"))
            {
                witBindgenPath += "\\";
            }

            foreach (var witFile in witFiles)
            {
                var startInfo = new ProcessStartInfo($"{witBindgenPath}wit-bindgen.exe",
                    $"c-sharp --out-dir {output} {witFile.Path}");

                startInfo.RedirectStandardOutput = true;
                startInfo.UseShellExecute = false;
                var witBindgenProcess = Process.Start(startInfo);
                witBindgenProcess.WaitForExit((int)TimeSpan.FromMinutes(1).TotalMilliseconds);
                //TODO : wit error handling / stdout / stderr
            }
        }

        public void Initialize(GeneratorInitializationContext context)
        {
            // No initialization required for this one
        }
    }
}
