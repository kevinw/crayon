use std::io::Read;
use std::sync::Arc;

use crayon::bincode;
use crayon::errors::*;
use crayon::res::location::Location;
use crayon::res::{ResourceHandle, ResourceLoader, ResourceSystemShared};

use super::prefab::*;
use super::WorldResourcesShared;

pub const MAGIC: [u8; 8] = [
    'P' as u8, 'R' as u8, 'E' as u8, 'B' as u8, ' ' as u8, 0, 0, 1,
];

pub struct PrefabLoader {
    world_resources: Arc<WorldResourcesShared>,
    res: Arc<ResourceSystemShared>,
}

impl PrefabLoader {
    pub fn new(res: Arc<ResourceSystemShared>, world_resources: Arc<WorldResourcesShared>) -> Self {
        PrefabLoader {
            res: res,
            world_resources: world_resources,
        }
    }
}

impl ResourceHandle for PrefabHandle {
    type Loader = PrefabLoader;
}

impl ResourceLoader for PrefabLoader {
    type Handle = PrefabHandle;

    fn create(&self) -> Result<Self::Handle> {
        let handle = self.world_resources.create_prefab_async();
        info!("[PrefabLoader] creates {:?}.", handle);
        Ok(handle)
    }

    fn load(&self, handle: Self::Handle, mut file: &mut dyn Read) -> Result<()> {
        let mut buf = [0; 8];
        file.read_exact(&mut buf[0..8])?;

        // magic: [u8; 8]
        if &buf[0..8] != &MAGIC[..] {
            bail!("[PrefabLoader] MAGIC number not match.");
        }

        let mut data: Prefab = bincode::deserialize_from(&mut file)?;
        for v in &data.universe_meshes {
            data.meshes.push(self.res.load_from(Location::from(*v))?);
        }

        for &v in &data.meshes {
            self.res.wait(v)?;
        }

        info!(
            "[PrefabLoader] loads {:?}. (Nodes: {}, Meshes: {})",
            handle,
            data.nodes.len(),
            data.meshes.len()
        );

        // The prefab handle might already been freed.
        if let Some(prefab) = self.world_resources.update_prefab_async(handle, data)? {
            for v in prefab.meshes {
                self.res.unload(v)?;
            }
        }

        Ok(())
    }

    fn delete(&self, handle: Self::Handle) -> Result<()> {
        info!("[PrefabLoader] deletes {:?}.", handle);

        if let Some(prefab) = self.world_resources.delete_prefab_async(handle) {
            for &v in &prefab.meshes {
                self.res.unload(v)?;
            }
        }

        Ok(())
    }
}
