//! Scene Mapping: Object ID → Render Object
//!
//! Maintains stable identity across frames.
//! If IDs flicker, visuals flicker.

use std::collections::HashMap;

/// Maps semantic object IDs to backend render IDs
///
/// Purpose: Maintain stable visual identity even as objects change
///
/// Why it matters:
/// - Prevents visual flickering from ID churn
/// - Enables efficient incremental updates
/// - Allows backend-specific ID management
pub struct SceneMap {
    /// Semantic ID → Render ID
    semantic_to_render: HashMap<u64, u64>,
    /// Render ID → Semantic ID (reverse lookup)
    render_to_semantic: HashMap<u64, u64>,
    /// Next available render ID
    next_render_id: u64,
}

impl SceneMap {
    /// Create empty scene map
    pub fn new() -> Self {
        Self {
            semantic_to_render: HashMap::new(),
            render_to_semantic: HashMap::new(),
            next_render_id: 1, // Start at 1, 0 is reserved for "no object"
        }
    }

    /// Insert a new mapping
    ///
    /// Returns the assigned render ID
    pub fn insert(&mut self, semantic_id: u64, render_id: u64) {
        self.semantic_to_render.insert(semantic_id, render_id);
        self.render_to_semantic.insert(render_id, semantic_id);
    }

    /// Allocate a new render ID for a semantic ID
    ///
    /// Automatically generates and tracks the next ID
    pub fn allocate(&mut self, semantic_id: u64) -> u64 {
        let render_id = self.next_render_id;
        self.next_render_id += 1;
        self.insert(semantic_id, render_id);
        render_id
    }

    /// Get render ID for semantic ID
    pub fn get(&self, semantic_id: u64) -> Option<u64> {
        self.semantic_to_render.get(&semantic_id).copied()
    }

    /// Get semantic ID for render ID
    pub fn get_semantic(&self, render_id: u64) -> Option<u64> {
        self.render_to_semantic.get(&render_id).copied()
    }

    /// Check if semantic ID exists
    pub fn contains(&self, semantic_id: u64) -> bool {
        self.semantic_to_render.contains_key(&semantic_id)
    }

    /// Remove a mapping by semantic ID
    ///
    /// Returns the removed render ID if it existed
    pub fn remove(&mut self, semantic_id: u64) -> Option<u64> {
        if let Some(render_id) = self.semantic_to_render.remove(&semantic_id) {
            self.render_to_semantic.remove(&render_id);
            Some(render_id)
        } else {
            None
        }
    }

    /// Remove a mapping by render ID
    pub fn remove_by_render_id(&mut self, render_id: u64) -> Option<u64> {
        if let Some(semantic_id) = self.render_to_semantic.remove(&render_id) {
            self.semantic_to_render.remove(&semantic_id);
            Some(semantic_id)
        } else {
            None
        }
    }

    /// Cleanup mappings based on predicate
    ///
    /// Returns render IDs that were removed
    ///
    /// Example: Remove objects that no longer exist in runtime
    /// ```
    /// # use renderer::scene_map::SceneMap;
    /// # use std::collections::HashSet;
    /// let mut scene_map = SceneMap::new();
    /// scene_map.allocate(100);
    /// scene_map.allocate(200);
    /// let current_ids: HashSet<u64> = [200].into_iter().collect();
    /// let removed = scene_map.cleanup(|id| !current_ids.contains(id));
    /// assert_eq!(removed.len(), 1);
    /// ```
    pub fn cleanup<F>(&mut self, mut should_remove: F) -> Vec<u64>
    where
        F: FnMut(&u64) -> bool,
    {
        let to_remove: Vec<u64> = self
            .semantic_to_render
            .keys()
            .filter(|id| should_remove(id))
            .copied()
            .collect();

        let mut removed_render_ids = Vec::new();

        for semantic_id in to_remove {
            if let Some(render_id) = self.remove(semantic_id) {
                removed_render_ids.push(render_id);
            }
        }

        removed_render_ids
    }

    /// Get all render IDs currently mapped
    pub fn all_render_ids(&self) -> Vec<u64> {
        self.render_to_semantic.keys().copied().collect()
    }

    /// Get all semantic IDs currently mapped
    pub fn all_semantic_ids(&self) -> Vec<u64> {
        self.semantic_to_render.keys().copied().collect()
    }

    /// Get number of mapped objects
    pub fn len(&self) -> usize {
        self.semantic_to_render.len()
    }

    /// Check if map is empty
    pub fn is_empty(&self) -> bool {
        self.semantic_to_render.is_empty()
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        self.semantic_to_render.clear();
        self.render_to_semantic.clear();
    }

    /// Get internal statistics for debugging
    pub fn stats(&self) -> SceneMapStats {
        SceneMapStats {
            total_objects: self.len(),
            next_render_id: self.next_render_id,
            semantic_capacity: self.semantic_to_render.capacity(),
            render_capacity: self.render_to_semantic.capacity(),
        }
    }
}

impl Default for SceneMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about scene map state
#[derive(Debug, Clone, Copy)]
pub struct SceneMapStats {
    pub total_objects: usize,
    pub next_render_id: u64,
    pub semantic_capacity: usize,
    pub render_capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_mapping() {
        let mut map = SceneMap::new();

        let semantic_id = 100;
        let render_id = map.allocate(semantic_id);

        assert_eq!(map.get(semantic_id), Some(render_id));
        assert_eq!(map.get_semantic(render_id), Some(semantic_id));
    }

    #[test]
    fn test_stable_ids() {
        let mut map = SceneMap::new();

        let id1 = map.allocate(100);
        let id2 = map.allocate(200);
        let id3 = map.allocate(300);

        // IDs should be sequential and stable
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_bidirectional_lookup() {
        let mut map = SceneMap::new();

        map.insert(100, 500);

        assert_eq!(map.get(100), Some(500));
        assert_eq!(map.get_semantic(500), Some(100));
    }

    #[test]
    fn test_removal() {
        let mut map = SceneMap::new();

        map.insert(100, 500);
        assert!(map.contains(100));

        let removed = map.remove(100);
        assert_eq!(removed, Some(500));
        assert!(!map.contains(100));
        assert_eq!(map.get_semantic(500), None);
    }

    #[test]
    fn test_cleanup() {
        let mut map = SceneMap::new();

        map.allocate(1);
        map.allocate(2);
        map.allocate(3);
        map.allocate(4);

        // Remove even IDs
        let removed = map.cleanup(|id| id % 2 == 0);

        assert_eq!(removed.len(), 2);
        assert!(map.contains(1));
        assert!(!map.contains(2));
        assert!(map.contains(3));
        assert!(!map.contains(4));
    }

    #[test]
    fn test_all_ids() {
        let mut map = SceneMap::new();

        map.allocate(100);
        map.allocate(200);
        map.allocate(300);

        let semantic = map.all_semantic_ids();
        let render = map.all_render_ids();

        assert_eq!(semantic.len(), 3);
        assert_eq!(render.len(), 3);
        assert!(semantic.contains(&100));
        assert!(semantic.contains(&200));
        assert!(semantic.contains(&300));
    }

    #[test]
    fn test_clear() {
        let mut map = SceneMap::new();

        map.allocate(100);
        map.allocate(200);
        assert_eq!(map.len(), 2);

        map.clear();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn test_no_id_collision() {
        let mut map = SceneMap::new();

        let id1 = map.allocate(100);
        map.remove(100);
        let id2 = map.allocate(200);

        // Even after removal, IDs keep incrementing
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_stats() {
        let mut map = SceneMap::new();

        map.allocate(1);
        map.allocate(2);

        let stats = map.stats();
        assert_eq!(stats.total_objects, 2);
        assert_eq!(stats.next_render_id, 3);
    }
}
