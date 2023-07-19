mod component_type_object;

use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use wit_component::StringEncoding;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    iter, mem,
    ops::Deref,
};
use wit_bindgen_core::{
    uwrite, uwriteln,
    wit_parser::{
        abi::{AbiVariant, Bindgen, Instruction, LiftLower, WasmType},
        Case, Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Int, InterfaceId, Record,
        Resolve, Result_, SizeAlign, Tuple, Type, TypeDef, TypeDefKind, TypeId, TypeOwner, Union,
        Variant, WorldId, WorldKey
    },
    Files, InterfaceGenerator as _, Ns, WorldGenerator,
};

//cargo run c-sharp --out-dir testing-csharp tests/codegen/floats.wit

const CSHARP_IMPORTS: &str = "\
using System;
using System.Runtime.CompilerServices;
using System.Collections;

using Wit.Native;
using Wit.Interop;\
";

const C_IMPORTS: &str = "\
#include <stdlib.h>

#include <mono-wasi/driver.h>
#include <mono/metadata/assembly.h>
#include <mono/metadata/class.h>
#include <mono/metadata/appdomain.h>
#include <mono/metadata/image.h>
#include <mono/metadata/metadata.h>
#include <mono/metadata/object.h>
#include <mono/metadata/debug-helpers.h>
#include <mono/metadata/reflection.h>
#include <mono/utils/mono-publib.h>\
";

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not to generate a stub class for exported functions
    #[cfg_attr(feature = "clap", arg(long))]
    pub string_encoding: StringEncoding,
    #[arg(short, long, default_value_t = false)]
    pub generate_stub: bool,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(CSharp {
            opts: self.clone(),
            ..CSharp::default()
        })
    }
}

enum Direction {
    Import,
    Export,
}

struct InterfaceFragment {
    csharp_src: String,
    csharp_interop_src: String,
    c_src: String,
    stub: String,
}

#[derive(Default)]
pub struct CSharp {
    opts: Opts,
    name: String,
    return_area_size: usize,
    return_area_align: usize,
    tuple_counts: HashSet<usize>,
    needs_result: bool,
    interface_fragments: HashMap<String, Vec<InterfaceFragment>>,
    world_fragments: Vec<InterfaceFragment>,
    sizes: SizeAlign,
    interface_names: HashMap<InterfaceId, String>,
}

impl CSharp {
    fn qualifier(&self) -> String {
        let world = self.name.to_upper_camel_case();
        format!("{world}World.")
    }

    fn interface<'a>(&'a mut self, resolve: &'a Resolve, name: &'a str) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: String::new(),
            c_src: String::new(),
            csharp_interop_src: String::new(),
            stub: String::new(),
            gen: self,
            resolve,
            name,
        }
    }
}

impl WorldGenerator for CSharp {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        let name = &resolve.worlds[world].name;
        self.name = name.to_string();
        self.sizes.fill(resolve);
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let name = interface_name(resolve, key, Direction::Import);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name);
        gen.types(id);

        // C
        uwriteln!(gen.c_src, "void attach_internal_calls() {{");
        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.import(&resolve.name_world_key(key), func);
        }
        uwriteln!(gen.c_src, "}}");

        gen.add_interface_fragment();
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = &format!("{}-world", resolve.worlds[world].name);
        let mut gen = self.interface(resolve, name);

        for (_, func) in funcs {
            gen.import(name, func);
        }

        gen.add_world_fragment();
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let name = interface_name(resolve, key, Direction::Export);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.export(func, Some(&resolve.name_world_key(key)));
        }

        gen.add_interface_fragment();
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = &format!("{}-world", resolve.worlds[world].name);
        let mut gen = self.interface(resolve, name);

        for (_, func) in funcs {
            gen.export(func, None);
        }

        gen.add_world_fragment();
    }

    fn export_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let name = &format!("{}-world", resolve.worlds[world].name);
        let mut gen = self.interface(resolve, name);

        for (ty_name, ty) in types {
            gen.define_type(ty_name, *ty);
        }

        gen.add_world_fragment();
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        let world = &resolve.worlds[id];
        let snake = world.name.to_snake_case();
        let namespace = format!("wit_{snake}");
        let name = world.name.to_upper_camel_case();

        let version = env!("CARGO_PKG_VERSION");
        let mut src = String::new();
        uwriteln!(src, "// Generated by `wit-bindgen` {version}. DO NOT EDIT!");

        uwrite!(
            src,
            "namespace {namespace};

             {CSHARP_IMPORTS}

             public static class {name}World {{
                private {name}World() {{}}
            "
        );

        src.push_str(
            &self
                .world_fragments
                .iter()
                .map(|f| f.csharp_src.deref())
                .collect::<Vec<_>>()
                .join("\n"),
        );

        let mut producers = wasm_metadata::Producers::empty();
        producers.add(
            "processed-by",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        );

        if self.needs_result {
            src.push_str(
                r#"
                using System.Runtime.InteropServices;

                namespace Wit.Interop;

                [StructLayout(LayoutKind.Sequential)]
                public readonly struct Result<Ok, Err>
                {
                    public readonly byte Tag;
                    private readonly object _value;

                    private Result(byte tag, object value)
                    {
                        Tag = tag;
                        _value = value;
                    }

                    public static Result<Ok, Err> ok(Ok ok)
                    {
                        return new Result<Ok, Err>(OK, ok!);
                    }

                    public static Result<Ok, Err> err(Err err)
                    {
                        return new Result<Ok, Err>(ERR, err!);
                    }

                    public bool IsOk => Tag == OK;
                    public bool IsErr => Tag == ERR;

                    public Ok AsOk 
                    { 
                        get 
                        {
                            if (Tag == OK) 
                                return (Ok)_value;
                            else 
                                throw new ArgumentException("expected OK, got " + Tag);
                        } 
                    }

                    public Err AsErr
                    {
                        get
                        {
                            if (Tag == ERR)
                                return (Err)_value;
                            else
                                throw new ArgumentException("expected ERR, got " + Tag);
                        }
                    }

                    public const byte OK = 0;
                    public const byte ERR = 1;
                }
                "#,
            )
        }

        src.push_str("}\n");

        files.push(&format!("{name}.cs"), indent(&src).as_bytes());

        let generate_stub = |name, fragments: &[InterfaceFragment], files: &mut Files| {
            let body = fragments
                .iter()
                .map(|f| f.stub.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let body = format!(
                "// Generated by `wit-bindgen` {version}. DO NOT EDIT!
                namespace {namespace};

                 {CSHARP_IMPORTS}

                 public class {name} {{
                     {body}
                 }}
                "
            );

            files.push(&format!("{name}.cs"), indent(&body).as_bytes());
            files.push(
                &format!("{snake}_component_type.o",),
                component_type_object::object(resolve, id, self.opts.string_encoding)
                    .unwrap()
                    .as_slice(),
            );
        };

        if self.opts.generate_stub {
            generate_stub(format!("{name}Impl"), &self.world_fragments, files);
        }

        // Imports for the .c file
        let c_name = self.name.to_snake_case();
        let mut c_src = format!(
            "
            // Generated by `wit-bindgen-csharp` {version}. DO NOT EDIT!
            {C_IMPORTS}
            
            #define DEFINE_DOTNET_METHOD(c_name, assembly_name, namespc, class_name, method_name) \
            MonoMethod* method_##c_name;\
            \
            __attribute__((export_name(c_name)))\
            MonoObject* c_name(MonoObject* target_instance, void* method_params[]) {{\
                if (!method_##c_name) {{\
                    method_##c_name = lookup_dotnet_method(assembly_name, namespc, class_name, method_name, -1);\
                    assert(method_##c_name);\
                }}\
            \
                MonoObject* exception;\
                MonoObject* res = mono_wasm_invoke_method(method_##c_name, target_instance, method_params, &exception);\
                assert(!exception);\
                return res;\
            }}

            "
        );

        for (name, fragments) in &self.interface_fragments {
            // C#
            let body = fragments
                .iter()
                .map(|f| f.csharp_src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let body = format!(
                "// Generated by `wit-bindgen` {version}. DO NOT EDIT!
                 namespace {namespace};

                 {CSHARP_IMPORTS}

                 public static class {name} {{
                     private {name}() {{}}

                     {body}
                 }}
                "
            );

            files.push(&format!("{name}.cs"), indent(&body).as_bytes());

            // C# Interop
            let body = fragments
                .iter()
                .map(|f| f.csharp_interop_src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let body = format!(
                "// Generated by `wit-bindgen` {version}. DO NOT EDIT!
                namespace {namespace};

                {CSHARP_IMPORTS}

                public static class {name}Interop {{
                    private {name}Interop() {{}}

                    {body}
                }}
                "
            );

            files.push(&format!("{name}Interop.cs"), indent(&body).as_bytes());

            // C
            let body = fragments
                .iter()
                .map(|f| f.c_src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            c_src.push_str(&body);
            c_src.push('\n');

            //TODO: Only generate stub for exports
            if self.opts.generate_stub {
                generate_stub(format!("{name}Impl"), fragments, files);
            }
        }

        // The initial PoC version we had, used the C generator and then we just manually changed the
        // bindings in the .c file to call the generated C# functions.
        // This is obiously not something we want to do, but we also do not want to essentially have two C generators
        // where one is C#/.NET specific if we can avoid it.
        // We could potentionally add a flag to the C generator to ommit parts of the generation that we would generate in the C# generator.
        // Alternatively we would add parameters to allow us to pass the information needed for the bindings and do the actual generation in the C generator.
        // Temporary "hack" to generate C bindings and bind those to the C# bindings.
        // TODO: Explore alternative ways of how we integrate with the C bindings.
        let mut c_generator = wit_bindgen_c::C::default();
        let mut c_files = Files::default();
        c_generator.generate(resolve, id, &mut c_files);

        let c_file_world_name = format!("{c_name}.c");

        for (c_file_name, contents) in c_files.iter() {
            if c_file_name == c_file_world_name {
                let mut contents = contents.to_vec();
                contents.extend_from_slice(indent(&c_src).as_bytes());

                files.push(c_file_name, &contents);
            } else {
                files.push(c_file_name, contents);
            }
        }
    }
}

struct InterfaceGenerator<'a> {
    src: String,
    c_src: String,
    csharp_interop_src: String,
    stub: String,
    gen: &'a mut CSharp,
    resolve: &'a Resolve,
    name: &'a str,
}

impl InterfaceGenerator<'_> {
    fn qualifier(&self, when: bool, ty: &TypeDef) -> String {
        if let TypeOwner::Interface(id) = &ty.owner {
            if let Some(name) = self.gen.interface_names.get(id) {
                if name != self.name {
                    return format!("{}.", name.to_upper_camel_case());
                }
            }
        }

        if when {
            let name = self.name.to_upper_camel_case();
            format!("{name}.")
        } else {
            String::new()
        }
    }

    fn add_interface_fragment(self) {
        self.gen
            .interface_fragments
            .entry(self.name.to_upper_camel_case())
            .or_default()
            .push(InterfaceFragment {
                csharp_src: self.src,
                c_src: self.c_src,
                csharp_interop_src: self.csharp_interop_src,
                stub: self.stub,
            });
    }

    fn add_world_fragment(self) {
        self.gen.world_fragments.push(InterfaceFragment {
            csharp_src: self.src,
            c_src: self.c_src,
            csharp_interop_src: self.csharp_interop_src,
            stub: self.stub,
        });
    }

    fn import(&mut self, module: &String, func: &Function) {
        if func.kind != FunctionKind::Freestanding {
            todo!("resources");
        }

        let mut bindgen = FunctionBindgen::new(
            self,
            &func.name,
            func.params
                .iter()
                .map(|(name, _)| name.to_csharp_ident())
                .collect(),
        );

        bindgen.gen.resolve.call(
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut bindgen,
        );

        let src = bindgen.src;

        let sig = self.resolve.wasm_signature(AbiVariant::GuestImport, func);

        let result_type = match &sig.results[..] {
            [] => "void",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let camel_name = func.name.to_upper_camel_case();
        let interop_name = format!("wasmImport{camel_name}");

        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = wasm_type(*param);
                format!("{ty} p{i}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let sig = self.sig_string(func, false);

        uwrite!(
            self.src,
            r#"{sig} {{
                   {src}
               }}
            "#
        );

        uwrite!(
            self.csharp_interop_src,
            r#"
                [MethodImpl(MethodImplOptions.InternalCall)]
                internal static extern unsafe {result_type} {interop_name}({params});

            "#
        );

        let c_module = module.to_snake_case();
        let c_fun_name = func.name.to_snake_case();

        let c_name = format!("{c_module}_{c_fun_name}");

        let csharp_module = module.to_upper_camel_case();

        uwrite!(
            self.c_src,
            r#"mono_add_internal_call("Wit.Native.{csharp_module}Interop::{interop_name}", {c_name});
            "#
        );
    }

    fn export(&mut self, func: &Function, interface_name: Option<&str>) {
        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);

        let export_name = func.core_export_name(interface_name);

        let mut bindgen = FunctionBindgen::new(
            self,
            &func.name,
            (0..sig.params.len()).map(|i| format!("p{i}")).collect(),
        );

        bindgen.gen.resolve.call(
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
        );

        assert!(!bindgen.needs_cleanup_list);

        let src = bindgen.src;

        let result_type = match &sig.results[..] {
            [] => "void",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let camel_name = func.name.to_upper_camel_case();

        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = wasm_type(*param);
                format!("{ty} p{i}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let untyped_params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, _)| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(", ");

        let interop_name = format!("wasmExport{camel_name}");
        let module = format!("{}", export_name.to_snake_case());
        let camel_module = format!("{}", export_name.to_lower_camel_case());

        uwrite!(
            self.csharp_interop_src,
            r#"internal static unsafe {result_type} {interop_name}({params}) {{
                {src}
            }}
            "#
        );

        uwrite!(
            self.src,
            r#"public static {result_type} {camel_name}({params})
                {{
                    {interop_name}({untyped_params});
                }}
            "#
        );

        uwrite!(
            self.c_src,
            r#"DEFINE_DOTNET_METHOD({module}, "{camel_module}.dll", "{module}.Wit.Native" , "Interop", "{interop_name}");
            "#
        );

        if self.gen.opts.generate_stub {
            let sig = self.sig_string(func, true);

            uwrite!(
                self.stub,
                r#"
                {sig} {{
                    throw new NotImplementedException();
                }}
                "#
            );
        }
    }

    fn type_name(&mut self, ty: &Type) -> String {
        self.type_name_with_qualifier(ty, false)
    }

    fn type_name_with_qualifier(&mut self, ty: &Type, qualifier: bool) -> String {
        match ty {
            Type::Bool => "bool".to_owned(),
            Type::U8 => "byte".to_owned(),
            Type::U16 => "ushort".to_owned(),
            Type::U32 => "uint".to_owned(),
            Type::U64 => "ulong".to_owned(),
            Type::S8 => "sbyte".to_owned(),
            Type::S16 => "short".to_owned(),
            Type::S32 => "int".to_owned(),
            Type::S64 => "long".to_owned(),
            Type::Float32 => "float".to_owned(),
            Type::Float64 => "double".to_owned(),
            Type::Char => "uint".to_owned(),
            Type::String => "string".to_owned(),
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.kind {
                    TypeDefKind::Type(ty) => self.type_name_with_qualifier(ty, qualifier),
                    TypeDefKind::List(ty) => {
                        if is_primitive(ty) {
                            format!("{}[]", self.type_name(ty))
                        } else {
                            format!("List<{}>", self.type_name_boxed(ty, qualifier))
                        }
                    }
                    TypeDefKind::Tuple(tuple) => {
                        let count = tuple.types.len();
                        self.gen.tuple_counts.insert(count);

                        let params = if count == 0 {
                            String::new()
                        } else {
                            format!(
                                "({})",
                                tuple
                                    .types
                                    .iter()
                                    .map(|ty| self.type_name_boxed(ty, qualifier))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        };

                        params
                    }
                    TypeDefKind::Option(ty) => self.type_name_boxed(ty, qualifier),
                    TypeDefKind::Result(result) => {
                        self.gen.needs_result = true;
                        let mut name = |ty: &Option<Type>| {
                            ty.as_ref()
                                .map(|ty| self.type_name_boxed(ty, qualifier))
                                .unwrap_or_else(|| "void".to_owned())
                        };
                        let ok = name(&result.ok);
                        let err = name(&result.err);

                        format!("{}Result<{ok}, {err}>", self.gen.qualifier())
                    }
                    _ => {
                        if let Some(name) = &ty.name {
                            format!(
                                "{}{}",
                                self.qualifier(qualifier, ty),
                                name.to_upper_camel_case()
                            )
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
        }
    }

    fn type_name_boxed(&mut self, ty: &Type, qualifier: bool) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::U8 => "byte".into(),
            Type::U16 => "ushort".into(),
            Type::U32 => "uint".into(),
            Type::U64 => "ulong".into(),
            Type::S8 => "sbyte".into(),
            Type::S16 => "short".into(),
            Type::S32 => "int".into(),
            Type::S64 => "long".into(),
            Type::Float32 => "float".into(),
            Type::Float64 => "double".into(),
            Type::Char => "uint".into(),
            Type::Id(id) => {
                let def = &self.resolve.types[*id];
                match &def.kind {
                    TypeDefKind::Type(ty) => self.type_name_boxed(ty, qualifier),
                    _ => self.type_name_with_qualifier(ty, qualifier),
                }
            }
            _ => self.type_name_with_qualifier(ty, qualifier),
        }
    }

    fn print_docs(&mut self, docs: &Docs) {
        if let Some(docs) = &docs.contents {
            let lines = docs
                .trim()
                .lines()
                .map(|line| format!("* {line}"))
                .collect::<Vec<_>>()
                .join("\n");

            uwrite!(
                self.src,
                "
                /**
                 {lines}
                 */
                "
            )
        }
    }

    fn non_empty_type<'a>(&self, ty: Option<&'a Type>) -> Option<&'a Type> {
        if let Some(ty) = ty {
            let id = match ty {
                Type::Id(id) => *id,
                _ => return Some(ty),
            };
            match &self.resolve.types[id].kind {
                TypeDefKind::Type(t) => self.non_empty_type(Some(t)).map(|_| ty),
                TypeDefKind::Record(r) => (!r.fields.is_empty()).then_some(ty),
                TypeDefKind::Tuple(t) => (!t.types.is_empty()).then_some(ty),
                _ => Some(ty),
            }
        } else {
            None
        }
    }

    fn sig_string(&mut self, func: &Function, qualifier: bool) -> String {
        let name = func.name.to_csharp_ident();

        let result_type = match func.results.len() {
            0 => "void".into(),
            1 => {
                self.type_name_with_qualifier(func.results.iter_types().next().unwrap(), qualifier)
            }
            count => {
                self.gen.tuple_counts.insert(count);
                format!(
                    "({})",
                    func.results
                        .iter_types()
                        .map(|ty| self.type_name_boxed(ty, qualifier))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        };

        let params = func
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.type_name_with_qualifier(ty, qualifier);
                let name = name.to_csharp_ident();
                format!("{ty} {name}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("public static {result_type} {name}({params})")
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let parameters = record
            .fields
            .iter()
            .map(|field| {
                format!(
                    "{} {}",
                    self.type_name(&field.ty),
                    field.name.to_csharp_ident()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let assignments = record
            .fields
            .iter()
            .map(|field| {
                let name = field.name.to_csharp_ident();
                format!("this.{name} = {name};")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let fields = if record.fields.is_empty() {
            format!("public const {name} INSTANCE = new {name}();")
        } else {
            record
                .fields
                .iter()
                .map(|field| {
                    format!(
                        "public readonly {} {};",
                        self.type_name(&field.ty),
                        field.name.to_csharp_ident()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        uwrite!(
            self.src,
            "
            public static class {name} {{
                {fields}

                public {name}({parameters}) {{
                    {assignments}
                }}
            }}
            "
        );
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let ty = match flags.repr() {
            FlagsRepr::U8 => "byte",
            FlagsRepr::U16 => "ushort",
            FlagsRepr::U32(1) => "uint",
            FlagsRepr::U32(2) => "ulong",
            repr => todo!("flags {repr:?}"),
        };

        let flags = flags
            .flags
            .iter()
            .enumerate()
            .map(|(i, flag)| {
                let flag_name = flag.name.to_shouty_snake_case();
                let suffix = if matches!(flags.repr(), FlagsRepr::U32(2)) {
                    "L"
                } else {
                    ""
                };
                format!(
                    "public static readonly {name} {flag_name} = new {name}(({ty}) (1{suffix} << {i}));"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            public static class {name} {{
                public readonly {ty} value;

                public {name}({ty} value) {{
                    this.value = value;
                }}

                {flags}
            }}
            "
        );
    }

    fn type_tuple(&mut self, id: TypeId, _name: &str, _tuple: &Tuple, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_variant(&mut self, _id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();
        let tag_type = int_type(variant.tag());

        let constructors = variant
            .cases
            .iter()
            .map(|case| {
                let case_name = case.name.to_csharp_ident();
                let tag = case.name.to_shouty_snake_case();
                let (parameter, argument) = if let Some(ty) = self.non_empty_type(case.ty.as_ref())
                {
                    (
                        format!("{} {case_name}", self.type_name(ty)),
                        case_name.deref(),
                    )
                } else {
                    (String::new(), "null")
                };

                format!(
                    "public static {name} {case_name}({parameter}) {{
                         return new {name}({tag}, {argument});
                     }}
                    "
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let accessors = variant
            .cases
            .iter()
            .filter_map(|case| {
                self.non_empty_type(case.ty.as_ref()).map(|ty| {
                    let case_name = case.name.to_upper_camel_case();
                    let tag = case.name.to_shouty_snake_case();
                    let ty = self.type_name(ty);
                    format!(
                        r#"public {ty} get{case_name}() {{
                               if (this.tag == {tag}) {{
                                   return ({ty}) this.value;
                               }} else {{
                                   throw new RuntimeException("expected {tag}, got " + this.tag);
                               }}
                           }}
                        "#
                    )
                })
            })
            .collect::<Vec<_>>()
            .join("\n");

        let tags = variant
            .cases
            .iter()
            .enumerate()
            .map(|(i, case)| {
                let tag = case.name.to_shouty_snake_case();
                format!("public readonly {tag_type} {tag} = {i};")
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            public static class {name} {{
                public readonly {tag_type} tag;
                private readonly Object value;

                private {name}({tag_type} tag, Object value) {{
                    this.tag = tag;
                    this.value = value;
                }}

                {constructors}
                {accessors}
                {tags}
            }}
            "
        );
    }

    fn type_option(&mut self, id: TypeId, _name: &str, _payload: &Type, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_result(&mut self, id: TypeId, _name: &str, _result: &Result_, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_union(&mut self, id: TypeId, name: &str, union: &Union, docs: &Docs) {
        self.type_variant(
            id,
            name,
            &Variant {
                cases: union
                    .cases
                    .iter()
                    .enumerate()
                    .map(|(i, case)| Case {
                        docs: case.docs.clone(),
                        name: format!("f{i}"),
                        ty: Some(case.ty),
                    })
                    .collect(),
            },
            docs,
        )
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let cases = enum_
            .cases
            .iter()
            .map(|case| case.name.to_shouty_snake_case())
            .collect::<Vec<_>>()
            .join(", ");

        uwrite!(
            self.src,
            "
            public static enum {name} {{
                {cases}
            }}
            "
        );
    }

    fn type_alias(&mut self, id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_list(&mut self, id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_builtin(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        unimplemented!();
    }

    fn define_type(&mut self, name: &str, id: TypeId) {
        let ty = &self.resolve().types[id];
        match &ty.kind {
            TypeDefKind::Record(record) => self.type_record(id, name, record, &ty.docs),
            TypeDefKind::Flags(flags) => self.type_flags(id, name, flags, &ty.docs),
            TypeDefKind::Tuple(tuple) => self.type_tuple(id, name, tuple, &ty.docs),
            TypeDefKind::Enum(enum_) => self.type_enum(id, name, enum_, &ty.docs),
            TypeDefKind::Variant(variant) => self.type_variant(id, name, variant, &ty.docs),
            TypeDefKind::Option(t) => self.type_option(id, name, t, &ty.docs),
            TypeDefKind::Result(r) => self.type_result(id, name, r, &ty.docs),
            TypeDefKind::Union(u) => self.type_union(id, name, u, &ty.docs),
            TypeDefKind::List(t) => self.type_list(id, name, t, &ty.docs),
            TypeDefKind::Type(t) => self.type_alias(id, name, t, &ty.docs),
            TypeDefKind::Future(_) => todo!("generate for future"),
            TypeDefKind::Stream(_) => todo!("generate for stream"),
            TypeDefKind::Resource => todo!("generate for resource"),
            TypeDefKind::Handle(_) => todo!("generate for handle"),
            TypeDefKind::Unknown => unreachable!(),
        }
    }
}

struct Block {
    body: String,
    results: Vec<String>,
    element: String,
    base: String,
}

struct BlockStorage {
    body: String,
    element: String,
    base: String,
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut InterfaceGenerator<'a>,
    func_name: &'b str,
    params: Box<[String]>,
    src: String,
    locals: Ns,
    block_storage: Vec<BlockStorage>,
    blocks: Vec<Block>,
    needs_cleanup_list: bool,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(
        gen: &'b mut InterfaceGenerator<'a>,
        func_name: &'b str,
        params: Box<[String]>,
    ) -> FunctionBindgen<'a, 'b> {
        Self {
            gen,
            func_name,
            params,
            src: String::new(),
            locals: Ns::default(),
            block_storage: Vec::new(),
            blocks: Vec::new(),
            needs_cleanup_list: false,
        }
    }
}

impl Bindgen for FunctionBindgen<'_, '_> {
    type Operand = String;

    fn emit(
        &mut self,
        _resolve: &Resolve,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(val.to_string()),
            Instruction::ConstZero { tys } => results.extend(tys.iter().map(|ty| {
                match ty {
                    WasmType::I32 => "0",
                    WasmType::I64 => "0L",
                    WasmType::F32 => "0.0F",
                    WasmType::F64 => "0.0D",
                }
                .to_owned()
            })),

            Instruction::I32Load { offset } => results.push(format!("returnArea.GetS32({offset})")),
            Instruction::I32Load8U { offset } => {
                results.push(format!("returnArea.GetU8({offset})"))
            }
            Instruction::I32Load8S { offset } => {
                results.push(format!("returnArea.GetS8({offset})"))
            }
            Instruction::I32Load16U { offset } => {
                results.push(format!("returnArea.GetU16({offset})"))
            }
            Instruction::I32Load16S { offset } => {
                results.push(format!("returnArea.GetS16({offset})"))
            }
            Instruction::I64Load { offset } => results.push(format!("returnArea.GetS64({offset})")),
            Instruction::F32Load { offset } => results.push(format!("returnArea.GetF32({offset})")),
            Instruction::F64Load { offset } => results.push(format!("returnArea.GetF64({offset})")),

            Instruction::I32Store { .. } => todo!("I32Store"),
            Instruction::I32Store8 { .. } => todo!("I32Store8"),
            Instruction::I32Store16 { .. } => todo!("I32Store16"),
            Instruction::I64Store { .. } => todo!("I64Store"),
            Instruction::F32Store { .. } => todo!("F32Store"),
            Instruction::F64Store { .. } => todo!("F64Store"),

            //This is handled in the C interface, so we just pass the value as is.
            Instruction::I32FromChar
            | Instruction::I64FromU64
            | Instruction::I64FromS64
            | Instruction::I32FromU32
            | Instruction::I32FromS32
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::F32FromFloat32
            | Instruction::F64FromFloat64
            | Instruction::S8FromI32
            | Instruction::U8FromI32
            | Instruction::S16FromI32
            | Instruction::U16FromI32
            | Instruction::S32FromI32
            | Instruction::U32FromI32
            | Instruction::S64FromI64
            | Instruction::U64FromI64
            | Instruction::CharFromI32
            | Instruction::Float32FromF32
            | Instruction::Float64FromF64 => results.push(operands[0].clone()),

            Instruction::Bitcasts { .. } => todo!("Bitcasts"),

            Instruction::I32FromBool => {
                results.push(format!("({} ? 1 : 0)", operands[0]));
            }
            Instruction::BoolFromI32 => results.push(format!("({} != 0)", operands[0])),

            Instruction::FlagsLower { .. } => todo!("FlagsLower"),

            Instruction::FlagsLift { .. } => todo!("FlagsLift"),

            Instruction::RecordLower { .. } => todo!("RecordLower"),
            Instruction::RecordLift { .. } => todo!("RecordLift"),
            Instruction::TupleLift { .. } => {
                let ops = operands
                    .iter()
                    .map(|op| op.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                results.push(format!("({ops})"));
            }

            Instruction::TupleLower { tuple: _, ty } => {
                let ops = operands
                    .iter()
                    .map(|op| op.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                results.push(format!("({ops})"));
                results.push(format!("({:?})", ty));
            }

            Instruction::VariantPayloadName => {
                todo!("VariantPayloadName");
            }

            Instruction::VariantLower { .. } => todo!("VariantLift"),

            Instruction::VariantLift { .. } => todo!("VariantLift"),

            Instruction::UnionLower { .. } => todo!("UnionLower"),

            Instruction::UnionLift { .. } => todo!("UnionLift"),

            Instruction::OptionLower { .. } => todo!("OptionLower"),

            Instruction::OptionLift { .. } => todo!("OptionLift"),

            Instruction::ResultLower { .. } => todo!("ResultLower"),

            Instruction::ResultLift { .. } => todo!("ResultLift"),

            Instruction::EnumLower { .. } => todo!("EnumLower"),

            Instruction::EnumLift { .. } => todo!("EnumLift"),

            Instruction::ListCanonLower { .. } => todo!("ListCanonLower"),

            Instruction::ListCanonLift { .. } => todo!("ListCanonLift"),

            Instruction::StringLower { realloc } => {
                let op = &operands[0];
                let interop_string = self.locals.tmp("interopString");
                uwriteln!(
                    self.src,
                    "InteropString {interop_string} = InteropString.FromString({op});"
                );

                //TODO: Oppertunity to optimize and not reallocate every call
                if realloc.is_none() {
                    results.push(format!("ref {interop_string}"));
                } else {
                    results.push(format!("ref {interop_string}"));
                }
                results.push(format!(""));
            }

            Instruction::StringLift { .. } => {
                let address = &operands[0];
                let length = &operands[1];

                results.push(format!("returnArea.GetUTF8String({address}, {length})"));
            }

            Instruction::ListLower { .. } => todo!("ListLower"),

            Instruction::ListLift { .. } => todo!("ListLift"),

            Instruction::IterElem { .. } => todo!("IterElem"),

            Instruction::IterBasePointer => todo!("IterBasePointer"),

            Instruction::CallWasm { sig, name } => {
                //TODO: Use base_name instead?
                let assignment = match &sig.results[..] {
                    [_] => {
                        let result = self.locals.tmp("result");
                        let assignment = format!("var {result} = ");
                        results.push(result);
                        assignment
                    }

                    [] => String::new(),

                    _ => unreachable!(),
                };

                let func_name = self.func_name.to_upper_camel_case();
                let name = name.to_upper_camel_case();

                let operands = operands.join(", ");

                uwriteln!(
                    self.src,
                    "{assignment} {name}Interop.wasmImport{func_name}({operands});"
                );
            }

            Instruction::CallInterface { func } => {
                let module = self.gen.name.to_upper_camel_case();
                let func_name = self.func_name.to_upper_camel_case();
                let name = module.to_upper_camel_case();

                let operands = operands.join(", ");
                if func.results.len() > 0 {
                    //uwriteln!(self.src, "{name}.{func_name}({operands});");
                    results.push(format!("{name}.{func_name}({operands})"));
                } else {
                    uwriteln!(self.src, "{name}.{func_name}({operands});");
                }
            }

            Instruction::Return { amt, .. } => match *amt {
                0 => (),
                1 => uwriteln!(self.src, "return {};", operands[0]),
                _ => {
                    let results = operands.join(", ");
                    uwriteln!(self.src, "return ({results});")
                }
            },

            Instruction::Malloc { .. } => unimplemented!(),

            Instruction::GuestDeallocate { .. } => todo!("GuestDeallocate"),

            Instruction::GuestDeallocateString => todo!("GuestDeallocateString"),

            Instruction::GuestDeallocateVariant { .. } => todo!("GuestDeallocateString"),

            Instruction::GuestDeallocateList { .. } => todo!("GuestDeallocateList"),
            Instruction::HandleLower { handle, name, ty } => todo!(),
            Instruction::HandleLift { handle, name, ty } => todo!("HandleLeft"),
        }
    }

    fn return_pointer(&mut self, size: usize, align: usize) -> String {
        self.gen.gen.return_area_size = self.gen.gen.return_area_size.max(size);
        self.gen.gen.return_area_align = self.gen.gen.return_area_align.max(align);
        format!("{}RETURN_AREA", self.gen.gen.qualifier())
    }

    fn push_block(&mut self) {
        self.block_storage.push(BlockStorage {
            body: mem::take(&mut self.src),
            element: self.locals.tmp("element"),
            base: self.locals.tmp("base"),
        });
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let BlockStorage {
            body,
            element,
            base,
        } = self.block_storage.pop().unwrap();

        self.blocks.push(Block {
            body: mem::replace(&mut self.src, body),
            results: mem::take(operands),
            element,
            base,
        });
    }

    fn sizes(&self) -> &SizeAlign {
        &self.gen.gen.sizes
    }

    fn is_list_canonical(&self, _resolve: &Resolve, element: &Type) -> bool {
        is_primitive(element)
    }
}

fn int_type(int: Int) -> &'static str {
    match int {
        Int::U8 => "byte",
        Int::U16 => "ushort",
        Int::U32 => "uint",
        Int::U64 => "ulong",
    }
}

fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "int",
        WasmType::I64 => "long",
        WasmType::F32 => "float",
        WasmType::F64 => "double",
    }
}

//TODO: Implement Flags
//fn flags_repr(flags: &Flags) -> Int {
//    match flags.repr() {
//        FlagsRepr::U8 => Int::U8,
//        FlagsRepr::U16 => Int::U16,
//        FlagsRepr::U32(1) => Int::U32,
//        FlagsRepr::U32(2) => Int::U64,
//        repr => panic!("unimplemented flags {repr:?}"),
//    }
//}

//fn list_element_info(ty: &Type) -> (usize, &'static str) {
//    match ty {
//        Type::S8 => (1, "sbyte"),
//        Type::S16 => (2, "short"),
//        Type::S32 => (4, "int"),
//        Type::S64 => (8, "long"),
//        Type::U8 => (1, "byte"),
//        Type::U16 => (2, "ushort"),
//        Type::U32 => (4, "uint"),
//        Type::U64 => (8, "ulong"),
//        Type::Float32 => (4, "float"),
//        Type::Float64 => (8, "double"),
//        _ => unreachable!(),
//    }
//}

fn indent(code: &str) -> String {
    let mut indented = String::with_capacity(code.len());
    let mut indent = 0;
    let mut was_empty = false;
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if was_empty {
                continue;
            }
            was_empty = true;
        } else {
            was_empty = false;
        }

        if trimmed.starts_with('}') {
            indent -= 1;
        }
        indented.extend(iter::repeat(' ').take(indent * 4));
        indented.push_str(trimmed);
        if trimmed.ends_with('{') {
            indent += 1;
        }
        indented.push('\n');
    }
    indented
}

fn world_name(resolve: &Resolve, world: WorldId) -> String {
    format!(
        "wit.worlds.{}",
        resolve.worlds[world].name.to_upper_camel_case()
    )
}

fn interface_name(resolve: &Resolve, name: &WorldKey, direction: Direction) -> String {
    let pkg = match name {
        WorldKey::Name(_) => None,
        WorldKey::Interface(id) => {
            let pkg = resolve.interfaces[*id].package.unwrap();
            Some(resolve.packages[pkg].name.clone())
        }
    };

    let name = match name {
        WorldKey::Name(name) => name,
        WorldKey::Interface(id) => resolve.interfaces[*id].name.as_ref().unwrap(),
    }
    .to_upper_camel_case();

    format!(
        "wit.{}.{}{name}",
        match direction {
            Direction::Import => "imports",
            Direction::Export => "exports",
        },
        if let Some(name) = &pkg {
            format!(
                "{}.{}.",
                name.namespace.to_csharp_ident(),
                name.name.to_csharp_ident()
            )
        } else {
            String::new()
        }
    )
}

fn is_primitive(ty: &Type) -> bool {
    matches!(
        ty,
        Type::U8
            | Type::S8
            | Type::U16
            | Type::S16
            | Type::U32
            | Type::S32
            | Type::U64
            | Type::S64
            | Type::Float32
            | Type::Float64
    )
}

trait ToCSharpIdent: ToOwned {
    fn to_csharp_ident(&self) -> Self::Owned;
}

impl ToCSharpIdent for str {
    fn to_csharp_ident(&self) -> String {
        // Escape C# keywords
        // Source: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/keywords/

        //TODO: Repace with actual keywords
        match self {
            "abstract" | "continue" | "for" | "new" | "switch" | "assert" | "default" | "goto"
            | "namespace" | "synchronized" | "boolean" | "do" | "if" | "private" | "this"
            | "break" | "double" | "implements" | "protected" | "throw" | "byte" | "else"
            | "import" | "public" | "throws" | "case" | "enum" | "instanceof" | "return"
            | "transient" | "catch" | "extends" | "int" | "short" | "try" | "char" | "final"
            | "interface" | "static" | "void" | "class" | "finally" | "long" | "strictfp"
            | "volatile" | "const" | "float" | "super" | "while" => format!("{self}_"),
            _ => self.to_lower_camel_case(),
        }
    }
}
