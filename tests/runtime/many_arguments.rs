use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!(in "tests/runtime/many_arguments");

#[derive(Default)]
pub struct MyImports {}

impl imports::Host for MyImports {
    fn many_arguments(
        &mut self,
        a1: u64,
        a2: u64,
        a3: u64,
        a4: u64,
        a5: u64,
        a6: u64,
        a7: u64,
        a8: u64,
        a9: u64,
        a10: u64,
        a11: u64,
        a12: u64,
        a13: u64,
        a14: u64,
        a15: u64,
        a16: u64,
    ) -> Result<()> {
        assert_eq!(a1, 1);
        assert_eq!(a2, 2);
        assert_eq!(a3, 3);
        assert_eq!(a4, 4);
        assert_eq!(a5, 5);
        assert_eq!(a6, 6);
        assert_eq!(a7, 7);
        assert_eq!(a8, 8);
        assert_eq!(a9, 9);
        assert_eq!(a10, 10);
        assert_eq!(a11, 11);
        assert_eq!(a12, 12);
        assert_eq!(a13, 13);
        assert_eq!(a14, 14);
        assert_eq!(a15, 15);
        assert_eq!(a16, 16);
        Ok(())
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "many_arguments",
        |linker| ManyArguments::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| ManyArguments::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(exports: ManyArguments, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_many_arguments(
        &mut *store,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
    )?;

    Ok(())
}
