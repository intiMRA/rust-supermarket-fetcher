use serde::{Deserialize, Serialize};
use crate::supermarkets::supermarket_types::Supermarket;

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Category {
    #[serde(default)]
    pub display_name: String,
    #[serde(alias = "name")]
    pub slug: String,
    #[serde(default)]
    pub children: Vec<Category>,
    #[serde(default)]
    pub supermarket: Supermarket
}

impl Category {
    pub fn flatten(&self) -> Vec<String> {
        let mut slugs = vec![self.slug.clone()];
        for child in &self.children {
            slugs.extend(child.flatten());
        }
        slugs
    }

    pub fn leaf_paths(&self, parent_path: &[String]) -> Vec<Vec<String>> {
        let mut current_path = parent_path.to_vec();
        current_path.push(self.slug.clone());

        if self.children.is_empty() {
            return vec![current_path];
        }

        let mut paths = Vec::new();
        for child in &self.children {
            paths.extend(child.leaf_paths(&current_path));
        }
        paths
    }

    pub fn get_trace(&self, category_slug: &str) -> Vec<String> {
        if self.slug == category_slug {
            return vec![self.slug.clone()];
        }
        for child in &self.children {
            let child_trace = child.get_trace(category_slug);
            if !child_trace.is_empty() {
                let mut trace = vec![self.slug.clone()];
                trace.extend(child_trace);
                return trace;
            }
        }
        Vec::new()
    }
}

pub fn flatten_categories(categories: &[Category]) -> Vec<String> {
    categories.into_iter().flat_map(|c| c.flatten()).collect()
}

pub fn leaf_category_paths(categories: &[Category]) -> Vec<Vec<String>> {
    categories.into_iter().flat_map(|c| c.leaf_paths(&[])).collect()
}

pub fn top_level_category_paths(categories: &[Category]) -> Vec<Vec<String>> {
    categories.into_iter().map(|c| vec![c.slug.clone()]).collect()
}

/// Returns category paths down to depth 1 (parent > child).
/// If a top-level category has children, returns one path per child.
/// If it has no children, returns just the top-level path.
pub fn child_category_paths(categories: &[Category]) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    for cat in categories {
        if cat.children.is_empty() {
            paths.push(vec![cat.slug.clone()]);
        } else {
            for child in &cat.children {
                paths.push(vec![cat.slug.clone(), child.slug.clone()]);
            }
        }
    }
    paths
}

pub fn find_trace(categories: &[Category], name: &str) -> Vec<String> {
    for category in categories {
        let trace = category.get_trace(name);
        if !trace.is_empty() {
            return trace;
        }
    }
    Vec::new()
}
