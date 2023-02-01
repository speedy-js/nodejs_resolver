use crate::{
    context::Context, description::PkgInfo, log::color, log::depth, normalize::NormalizePath,
    AliasMap, Info, PathKind, Plugin, ResolveResult, Resolver, State,
};
use std::path::Path;

pub struct BrowserFieldPlugin<'a> {
    pkg_info: &'a PkgInfo,
}

impl<'a> BrowserFieldPlugin<'a> {
    pub fn new(pkg_info: &'a PkgInfo) -> Self {
        Self { pkg_info }
    }

    fn request_target_is_module_and_equal_alias_key(alias_key: &String, info: &Info) -> bool {
        info.request().target().eq(alias_key)
    }

    fn request_path_is_equal_alias_key_path(
        alias_path: &Path,
        info: &Info,
        extensions: &[String],
    ) -> bool {
        let alias_path = alias_path.normalize();
        let request_path = info.to_resolved_path();
        let request_path = request_path.normalize();
        alias_path.eq(&request_path)
            || extensions.iter().any(|ext| {
                let path_with_extension = Resolver::append_ext_for_path(&request_path, ext);
                alias_path.eq(&path_with_extension)
            })
    }
}

impl<'a> Plugin for BrowserFieldPlugin<'a> {
    fn apply(&self, resolver: &Resolver, info: Info, context: &mut Context) -> State {
        if !resolver.options.browser_field {
            return State::Resolving(info);
        }
        for (alias_key, alias_target) in &self.pkg_info.json.alias_fields {
            let should_deal_alias = match matches!(info.request().kind(), PathKind::Normal) {
                true => Self::request_target_is_module_and_equal_alias_key(alias_key, &info),
                false => Self::request_path_is_equal_alias_key_path(
                    &self.pkg_info.dir_path.join(alias_key),
                    &info,
                    &resolver.options.extensions,
                ),
            };
            if !should_deal_alias {
                continue;
            }
            tracing::debug!(
                "BrowserFiled in '{}' works, trigger by '{}'({})",
                color::blue(&format!(
                    "{}/package.json",
                    self.pkg_info.dir_path.display()
                )),
                color::blue(alias_key),
                depth(&context.depth)
            );
            match alias_target {
                AliasMap::Target(converted) => {
                    if alias_key == converted {
                        // pointed itself in `browser` field:
                        // {
                        //  "recursive": "recursive"
                        // }
                        return State::Resolving(info);
                    }
                    let alias_info = Info::new(
                        &self.pkg_info.dir_path,
                        info.request().clone().with_target(converted),
                    );
                    let state = resolver._resolve(alias_info, context);
                    if state.is_finished() {
                        return state;
                    }
                    tracing::debug!("Leaving BrowserFiled({})", depth(&context.depth));
                }
                AliasMap::Ignored => return State::Success(ResolveResult::Ignored),
            };
        }
        State::Resolving(info)
    }
}
