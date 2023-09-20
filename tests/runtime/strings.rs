use crate::TestConfigurer;
use crate::Wasi;
use anyhow::Result;
use wasmtime::component::__internal::async_trait;
use wasmtime::component::{Component, Instance, Linker};
use wasmtime::Store;

wasmtime::component::bindgen!({
    path : "tests/runtime/strings",
    async: true,
});

#[derive(Default)]
pub struct MyImports;

#[async_trait]
impl test::strings::imports::Host for MyImports {
    async fn take_basic(&mut self, s: String) -> Result<()> {
        assert_eq!(s, "latin utf16");
        Ok(())
    }

    async fn return_unicode(&mut self) -> Result<String> {
        Ok("🚀🚀🚀 𠈄𓀀".to_string())
    }
}

struct StringsConfigurer {}

#[async_trait]
impl TestConfigurer<MyImports, Strings> for StringsConfigurer {
    async fn instantiate_async(
        &self,
        store: &mut Store<Wasi<MyImports>>,
        component: &Component,
        linker: &Linker<Wasi<MyImports>>,
    ) -> Result<(Strings, Instance)> {
        Strings::instantiate_async(store, component, linker).await
    }

    async fn test(&self, exports: Strings, store: &mut Store<Wasi<MyImports>>) -> Result<()> {
        run_test(exports, store).await
    }
}

#[tokio::test]
async fn run() -> Result<()> {
    let configurer = StringsConfigurer {};

    crate::run_test(
        "strings",
        |linker| Strings::add_to_linker(linker, |x| &mut x.0),
        configurer,
    )
    .await
}

async fn run_test(exports: Strings, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_test_imports(&mut *store).await?;
    assert_eq!(exports.call_return_empty(&mut *store).await?, "");
    assert_eq!(exports.call_roundtrip(&mut *store, "str").await?, "str");
    assert_eq!(
        exports.call_roundtrip(&mut *store, "🚀🚀🚀 𠈄𓀀").await?,
        "🚀🚀🚀 𠈄𓀀"
    );
    Ok(())
}
