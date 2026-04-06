use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, Ident, LitStr, Token,
};

// ── DSL 数据结构 ────────────────────────────────────────────────

/// 顶层宏输入。
struct NodeMacroInput {
    name: LitStr,
    title: LitStr,
    category: LitStr,
    inputs: Vec<PinInput>,
    outputs: Vec<PinInput>,
    params: Vec<ParamInput>,
    exec_ctx: Ident,
    exec_inputs: Ident,
    exec_body: TokenStream2,
}

/// 引脚定义（inputs / outputs 中的一项）。
struct PinInput {
    name: Ident,
    data_type: Ident,
    required: bool,
}

/// 参数定义（params 中的一项）。
struct ParamInput {
    name: Ident,
    data_type: Ident,
    constraint: Option<ConstraintInput>,
    default_expr: Expr,
}

/// 约束定义。
enum ConstraintInput {
    Range(Expr, Expr),
    FilePath(Vec<LitStr>),
}

// ── 解析实现 ────────────────────────────────────────────────────

impl Parse for NodeMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut title: Option<LitStr> = None;
        let mut category: Option<LitStr> = None;
        let mut inputs: Option<Vec<PinInput>> = None;
        let mut outputs: Option<Vec<PinInput>> = None;
        let mut params: Option<Vec<ParamInput>> = None;
        let mut exec_ctx: Option<Ident> = None;
        let mut exec_inputs: Option<Ident> = None;
        let mut exec_body: Option<TokenStream2> = None;

        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match field_name.to_string().as_str() {
                "name" => {
                    name = Some(input.parse()?);
                }
                "title" => {
                    title = Some(input.parse()?);
                }
                "category" => {
                    category = Some(input.parse()?);
                }
                "inputs" => {
                    inputs = Some(parse_pin_list(input)?);
                }
                "outputs" => {
                    outputs = Some(parse_pin_list(input)?);
                }
                "params" => {
                    params = Some(parse_param_list(input)?);
                }
                "execute" => {
                    let (ctx, inp, body) = parse_execute(input)?;
                    exec_ctx = Some(ctx);
                    exec_inputs = Some(inp);
                    exec_body = Some(body);
                }
                other => {
                    return Err(syn::Error::new(
                        field_name.span(),
                        format!("未知字段: `{other}`"),
                    ));
                }
            }

            // 可选的尾逗号
            let _ = input.parse::<Token![,]>();
        }

        let missing = |name: &str| {
            syn::Error::new(proc_macro2::Span::call_site(), format!("缺少字段: `{name}`"))
        };

        Ok(NodeMacroInput {
            name: name.ok_or_else(|| missing("name"))?,
            title: title.ok_or_else(|| missing("title"))?,
            category: category.ok_or_else(|| missing("category"))?,
            inputs: inputs.ok_or_else(|| missing("inputs"))?,
            outputs: outputs.ok_or_else(|| missing("outputs"))?,
            params: params.ok_or_else(|| missing("params"))?,
            exec_ctx: exec_ctx.ok_or_else(|| missing("execute"))?,
            exec_inputs: exec_inputs.ok_or_else(|| missing("execute"))?,
            exec_body: exec_body.ok_or_else(|| missing("execute"))?,
        })
    }
}

/// 解析引脚列表 `[name: Type required, name: Type, ...]`
fn parse_pin_list(input: ParseStream) -> syn::Result<Vec<PinInput>> {
    let content;
    bracketed!(content in input);
    let mut pins = Vec::new();
    while !content.is_empty() {
        let name: Ident = content.parse()?;
        content.parse::<Token![:]>()?;
        let data_type: Ident = content.parse()?;

        // 检查 `required` 关键字
        let required = if content.peek(Ident) {
            let kw: Ident = content.fork().parse()?;
            if kw == "required" {
                let _: Ident = content.parse()?;
                true
            } else {
                false
            }
        } else {
            false
        };

        pins.push(PinInput {
            name,
            data_type,
            required,
        });

        // 可选逗号
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(pins)
}

/// 解析参数列表 `[name: Type constraint? default(expr), ...]`
fn parse_param_list(input: ParseStream) -> syn::Result<Vec<ParamInput>> {
    let content;
    bracketed!(content in input);
    let mut params = Vec::new();
    while !content.is_empty() {
        let name: Ident = content.parse()?;
        content.parse::<Token![:]>()?;
        let data_type: Ident = content.parse()?;

        // 约束和 default 的解析：peek 下一个 ident
        let mut constraint = None;
        let mut default_expr: Option<Expr> = None;

        // 循环解析可能出现的 constraint 和 default
        while content.peek(Ident) {
            let kw: Ident = content.fork().parse()?;
            match kw.to_string().as_str() {
                "range" => {
                    let _: Ident = content.parse()?;
                    let inner;
                    parenthesized!(inner in content);
                    let min: Expr = inner.parse()?;
                    inner.parse::<Token![,]>()?;
                    let max: Expr = inner.parse()?;
                    constraint = Some(ConstraintInput::Range(min, max));
                }
                "file_path" => {
                    let _: Ident = content.parse()?;
                    let inner;
                    parenthesized!(inner in content);
                    let mut extensions = Vec::new();
                    while !inner.is_empty() {
                        extensions.push(inner.parse::<LitStr>()?);
                        if inner.peek(Token![,]) {
                            inner.parse::<Token![,]>()?;
                        }
                    }
                    constraint = Some(ConstraintInput::FilePath(extensions));
                }
                "default" => {
                    let _: Ident = content.parse()?;
                    let inner;
                    parenthesized!(inner in content);
                    default_expr = Some(inner.parse()?);
                }
                _ => break,
            }
        }

        let default_expr = default_expr.ok_or_else(|| {
            syn::Error::new(name.span(), format!("参数 `{}` 缺少 default(…)", name))
        })?;

        params.push(ParamInput {
            name,
            data_type,
            constraint,
            default_expr,
        });

        // 可选逗号
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(params)
}

/// 解析 execute 闭包 `|ctx, inputs| { body }`
fn parse_execute(input: ParseStream) -> syn::Result<(Ident, Ident, TokenStream2)> {
    input.parse::<Token![|]>()?;
    let ctx: Ident = input.parse()?;
    input.parse::<Token![,]>()?;
    let inputs: Ident = input.parse()?;
    input.parse::<Token![|]>()?;

    let body_content;
    braced!(body_content in input);
    let body: TokenStream2 = body_content.parse()?;

    Ok((ctx, inputs, body))
}

// ── 代码生成 ────────────────────────────────────────────────────

/// 将类型标识符（如 `Image`）映射为 `DataType::xxx()` 调用。
fn data_type_tokens(ident: &Ident) -> TokenStream2 {
    match ident.to_string().as_str() {
        "Image" => quote! { types::DataType::image() },
        "Float" => quote! { types::DataType::float() },
        "Int" => quote! { types::DataType::int() },
        "Bool" => quote! { types::DataType::bool() },
        "Color" => quote! { types::DataType::color() },
        "String" => quote! { types::DataType::string() },
        other => {
            let msg = format!("不支持的数据类型: `{other}`");
            quote! { compile_error!(#msg) }
        }
    }
}

/// 将默认值表达式根据类型包装为 `Value::Xxx(expr)`。
fn default_value_tokens(data_type: &Ident, expr: &Expr) -> TokenStream2 {
    match data_type.to_string().as_str() {
        "Float" => quote! { types::Value::Float(#expr) },
        "Int" => quote! { types::Value::Int(#expr) },
        "Bool" => quote! { types::Value::Bool(#expr) },
        "String" => quote! { types::Value::String((#expr).to_string()) },
        "Image" => quote! { compile_error!("Image 类型不支持 default 值") },
        "Color" => quote! { compile_error!("Color 类型不支持 default 值") },
        other => {
            let msg = format!("不支持的参数类型: `{other}`");
            quote! { compile_error!(#msg) }
        }
    }
}

/// 将约束转换为 `Some(Constraint::xxx(…))` 或 `None`。
fn constraint_tokens(constraint: &Option<ConstraintInput>) -> TokenStream2 {
    match constraint {
        None => quote! { None },
        Some(ConstraintInput::Range(min, max)) => {
            quote! { Some(types::Constraint::range(#min, #max)) }
        }
        Some(ConstraintInput::FilePath(exts)) => {
            quote! {
                Some(types::Constraint::file_path(
                    vec![#( #exts.into() ),*]
                ))
            }
        }
    }
}

/// 生成单个 PinDef 的 token。
fn pin_def_tokens(pin: &PinInput) -> TokenStream2 {
    let name_str = pin.name.to_string();
    let dt = data_type_tokens(&pin.data_type);
    let optional = !pin.required;
    quote! {
        crate::registry::PinDef {
            name: #name_str.to_string(),
            data_type: #dt,
            optional: #optional,
        }
    }
}

/// 生成单个 ParamDef 的 token。
fn param_def_tokens(param: &ParamInput) -> TokenStream2 {
    let name_str = param.name.to_string();
    let dt = data_type_tokens(&param.data_type);
    let constraint = constraint_tokens(&param.constraint);
    let default = default_value_tokens(&param.data_type, &param.default_expr);
    quote! {
        crate::registry::ParamDef {
            name: #name_str.to_string(),
            data_type: #dt,
            constraint: #constraint,
            default_value: #default,
        }
    }
}

// ── 入口 ────────────────────────────────────────────────────────

#[proc_macro]
pub fn node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NodeMacroInput);

    let name = &input.name;
    let title = &input.title;
    let category = &input.category;

    let input_defs: Vec<TokenStream2> = input.inputs.iter().map(pin_def_tokens).collect();
    let output_defs: Vec<TokenStream2> = input.outputs.iter().map(pin_def_tokens).collect();
    let param_defs: Vec<TokenStream2> = input.params.iter().map(param_def_tokens).collect();

    let ctx_ident = &input.exec_ctx;
    let inputs_ident = &input.exec_inputs;
    let body = &input.exec_body;

    let expanded = quote! {
        inventory::submit!(crate::registry::NodeDefEntry(|| {
            crate::registry::NodeDef {
                type_id: #name.to_string(),
                name: #title.to_string(),
                category: #category.to_string(),
                inputs: vec![
                    #( #input_defs ),*
                ],
                outputs: vec![
                    #( #output_defs ),*
                ],
                params: vec![
                    #( #param_defs ),*
                ],
                execute: Box::new(|#ctx_ident, #inputs_ident| {
                    let __node_result = {
                        #body
                    };
                    Box::pin(async move { __node_result })
                }),
            }
        }));
    };

    expanded.into()
}
