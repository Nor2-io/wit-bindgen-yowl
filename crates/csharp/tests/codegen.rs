//TODO: Implement tests similar to the other generators.
// This requires that we have any dependencies either included here or published to NuGet or similar.
use wit_component::StringEncoding;
use std::path::{Path};

macro_rules! codegen_test {
    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-csharp",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_csharp::Opts {
                        generate_stub: true,
                        string_encoding: StringEncoding::UTF8
                    }
                    .build()
                    .generate(resolve, world, files)
                },
                verify,
            )
        }
    };
}
test_helpers::codegen_tests!();

fn verify(_dir: &Path, _name: &str) {
// TODO?
}