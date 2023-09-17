use crate::TestConfigurer;
use crate::Wasi;
use anyhow::Result;
use wasmtime::component::__internal::async_trait;
use wasmtime::component::{Component, Instance, Linker};
use wasmtime::Store;
wasmtime::component::bindgen!({
    path: "tests/runtime/numbers",
    async : true,
});
#[derive(Default)]
pub struct MyImports {
    scalar: u32,
}
#[async_trait]
impl test::numbers::test::Host for MyImports {
    async fn roundtrip_u8(&mut self, val: u8) -> Result<u8> {
        Ok(val)
    }
    async fn roundtrip_s8(&mut self, val: i8) -> Result<i8> {
        Ok(val)
    }
    async fn roundtrip_u16(&mut self, val: u16) -> Result<u16> {
        Ok(val)
    }
    async fn roundtrip_s16(&mut self, val: i16) -> Result<i16> {
        Ok(val)
    }
    async fn roundtrip_u32(&mut self, val: u32) -> Result<u32> {
        Ok(val)
    }
    async fn roundtrip_s32(&mut self, val: i32) -> Result<i32> {
        Ok(val)
    }
    async fn roundtrip_u64(&mut self, val: u64) -> Result<u64> {
        Ok(val)
    }
    async fn roundtrip_s64(&mut self, val: i64) -> Result<i64> {
        Ok(val)
    }
    async fn roundtrip_float32(&mut self, val: f32) -> Result<f32> {
        Ok(val)
    }
    async fn roundtrip_float64(&mut self, val: f64) -> Result<f64> {
        Ok(val)
    }
    async fn roundtrip_char(&mut self, val: char) -> Result<char> {
        Ok(val)
    }
    async fn set_scalar(&mut self, val: u32) -> Result<()> {
        self.scalar = val;
        Ok(())
    }

    async fn get_scalar(&mut self) -> Result<u32> {
        Ok(self.scalar)
    }
}
struct NumbersConfigurer {}
#[async_trait]
impl TestConfigurer<MyImports, Numbers> for NumbersConfigurer {
    async fn instantiate_async(
        &self,
        store: &mut Store<Wasi<MyImports>>,
        component: &Component,
        linker: &Linker<Wasi<MyImports>>,
    ) -> Result<(Numbers, Instance)> {
        Numbers::instantiate_async(store, component, linker).await
    }
    async fn test(&self, exports: Numbers, store: &mut Store<Wasi<MyImports>>) -> Result<()> {
        run_test(exports, store).await
    }
}
#[tokio::test]
async fn run() -> Result<()> {
    let configurer = NumbersConfigurer {};
    crate::run_test(
        "numbers",
        |linker| Numbers::add_to_linker(linker, |x| &mut x.0),
        configurer,
    )
    .await
}
async fn run_test(exports: Numbers, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_test_imports(&mut *store).await?;
    let exports = exports.test_numbers_test();
    assert_eq!(exports.call_roundtrip_u8(&mut *store, 1).await?, 1);
    assert_eq!(
        exports
            .call_roundtrip_u8(&mut *store, u8::min_value())
            .await?,
        u8::min_value()
    );
    assert_eq!(
        exports
            .call_roundtrip_u8(&mut *store, u8::max_value())
            .await?,
        u8::max_value()
    );
    assert_eq!(exports.call_roundtrip_s8(&mut *store, 1).await?, 1);
    assert_eq!(
        exports
            .call_roundtrip_s8(&mut *store, i8::min_value())
            .await?,
        i8::min_value()
    );
    assert_eq!(
        exports
            .call_roundtrip_s8(&mut *store, i8::max_value())
            .await?,
        i8::max_value()
    );
    assert_eq!(exports.call_roundtrip_u16(&mut *store, 1).await?, 1);
    assert_eq!(
        exports
            .call_roundtrip_u16(&mut *store, u16::min_value())
            .await?,
        u16::min_value()
    );
    assert_eq!(
        exports
            .call_roundtrip_u16(&mut *store, u16::max_value())
            .await?,
        u16::max_value()
    );
    assert_eq!(exports.call_roundtrip_s16(&mut *store, 1).await?, 1);
    assert_eq!(
        exports
            .call_roundtrip_s16(&mut *store, i16::min_value())
            .await?,
        i16::min_value()
    );
    assert_eq!(
        exports
            .call_roundtrip_s16(&mut *store, i16::max_value())
            .await?,
        i16::max_value()
    );
    assert_eq!(exports.call_roundtrip_u32(&mut *store, 1).await?, 1);
    assert_eq!(
        exports
            .call_roundtrip_u32(&mut *store, u32::min_value())
            .await?,
        u32::min_value()
    );
    assert_eq!(
        exports
            .call_roundtrip_u32(&mut *store, u32::max_value())
            .await?,
        u32::max_value()
    );
    assert_eq!(exports.call_roundtrip_s32(&mut *store, 1).await?, 1);
    assert_eq!(
        exports
            .call_roundtrip_s32(&mut *store, i32::min_value())
            .await?,
        i32::min_value()
    );
    assert_eq!(
        exports
            .call_roundtrip_s32(&mut *store, i32::max_value())
            .await?,
        i32::max_value()
    );
    assert_eq!(exports.call_roundtrip_u64(&mut *store, 1).await?, 1);
    assert_eq!(
        exports
            .call_roundtrip_u64(&mut *store, u64::min_value())
            .await?,
        u64::min_value()
    );
    assert_eq!(
        exports
            .call_roundtrip_u64(&mut *store, u64::max_value())
            .await?,
        u64::max_value()
    );
    assert_eq!(exports.call_roundtrip_s64(&mut *store, 1).await?, 1);
    assert_eq!(
        exports
            .call_roundtrip_s64(&mut *store, i64::min_value())
            .await?,
        i64::min_value()
    );
    assert_eq!(
        exports
            .call_roundtrip_s64(&mut *store, i64::max_value())
            .await?,
        i64::max_value()
    );
    assert_eq!(exports.call_roundtrip_float32(&mut *store, 1.0).await?, 1.0);
    assert_eq!(
        exports
            .call_roundtrip_float32(&mut *store, f32::INFINITY)
            .await?,
        f32::INFINITY
    );
    assert_eq!(
        exports
            .call_roundtrip_float32(&mut *store, f32::NEG_INFINITY)
            .await?,
        f32::NEG_INFINITY
    );
    assert!(exports
        .call_roundtrip_float32(&mut *store, f32::NAN)
        .await?
        .is_nan());
    assert_eq!(exports.call_roundtrip_float64(&mut *store, 1.0).await?, 1.0);
    assert_eq!(
        exports
            .call_roundtrip_float64(&mut *store, f64::INFINITY)
            .await?,
        f64::INFINITY
    );
    assert_eq!(
        exports
            .call_roundtrip_float64(&mut *store, f64::NEG_INFINITY)
            .await?,
        f64::NEG_INFINITY
    );
    assert!(exports
        .call_roundtrip_float64(&mut *store, f64::NAN)
        .await?
        .is_nan());
    assert_eq!(exports.call_roundtrip_char(&mut *store, 'a').await?, 'a');
    assert_eq!(exports.call_roundtrip_char(&mut *store, ' ').await?, ' ');
    assert_eq!(exports.call_roundtrip_char(&mut *store, '🚩').await?, '🚩');
    exports.call_set_scalar(&mut *store, 2).await?;
    assert_eq!(exports.call_get_scalar(&mut *store).await?, 2);
    exports.call_set_scalar(&mut *store, 4).await?;
    assert_eq!(exports.call_get_scalar(&mut *store).await?, 4);
    Ok(())
}
