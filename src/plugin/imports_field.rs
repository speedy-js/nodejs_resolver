use std::sync::Arc;

use crate::{description::PkgFileInfo, plugin::ExportsFieldPlugin, Resolver, MODULE};

use super::{AliasFieldPlugin, Plugin};
use crate::{
    map::{Field, ImportsField},
    PathKind, ResolverInfo, ResolverStats,
};

pub struct ImportsFieldPlugin<'a> {
    pkg_info: &'a Option<Arc<PkgFileInfo>>,
}

impl<'a> ImportsFieldPlugin<'a> {
    pub fn new(pkg_info: &'a Option<Arc<PkgFileInfo>>) -> Self {
        Self { pkg_info }
    }
}

impl<'a> Plugin for ImportsFieldPlugin<'a> {
    fn apply(&self, resolver: &Resolver, info: ResolverInfo) -> ResolverStats {
        if let Some(pkg_info) = self.pkg_info {
            if !info.request.target.starts_with('#') {
                return ResolverStats::Resolving(info);
            }

            let target = &info.request.target;
            let list = if let Some(root) = &pkg_info.imports_field_tree {
                match ImportsField::field_process(root, target, &resolver.options.condition_names) {
                    Ok(list) => list,
                    Err(err) => return ResolverStats::Error((err, info)),
                }
            } else {
                return ResolverStats::Resolving(info);
            };

            assert!(list.len() <= 1); // TODO: need to confirm it.

            if let Some(item) = list.first() {
                let request = resolver.parse(item);
                let is_normal_kind = matches!(request.kind, PathKind::Normal);
                let info = ResolverInfo::from(
                    if is_normal_kind {
                        pkg_info.abs_dir_path.join(MODULE)
                    } else {
                        pkg_info.abs_dir_path.to_path_buf()
                    },
                    request,
                );

                let path = info.get_path();
                let info = if is_normal_kind {
                    // TODO: should optimized
                    let pkg_info = match resolver.load_pkg_file(&path) {
                        Ok(info) => info,
                        Err(err) => return ResolverStats::Error((err, info)),
                    };
                    if let Some(ref pkg_info) = pkg_info {
                        if !pkg_info.abs_dir_path.display().to_string().contains(MODULE) {
                            return ResolverStats::Resolving(info);
                        }
                    }

                    let stats = ExportsFieldPlugin::new(&pkg_info)
                        .apply(resolver, info)
                        .and_then(|info| ImportsFieldPlugin::new(&pkg_info).apply(resolver, info))
                        .and_then(|info| AliasFieldPlugin::new(&pkg_info).apply(resolver, info));
                    if let ResolverStats::Resolving(info) = stats {
                        info
                    } else {
                        return stats;
                    }
                } else {
                    info
                };
                ResolverStats::Resolving(info)
            } else {
                ResolverStats::Error((format!("Package path {target} is not exported"), info))
            }
        } else {
            ResolverStats::Resolving(info)
        }
    }
}
