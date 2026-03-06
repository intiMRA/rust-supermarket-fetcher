use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Category {
    pub name: String,
    #[serde(default)]
    pub children: Vec<Category>,
}

impl Category {
    pub fn flatten(&self) -> Vec<String> {
        let mut names = vec![self.name.clone()];
        for child in &self.children {
            names.extend(child.flatten());
        }
        names
    }

    pub fn flatten_paths(&self, parent_path: &[String]) -> Vec<Vec<String>> {
        let mut current_path = parent_path.to_vec();
        current_path.push(self.name.clone());

        let mut paths = vec![current_path.clone()];
        for child in &self.children {
            paths.extend(child.flatten_paths(&current_path));
        }
        paths
    }

    pub fn get_trace(&self, category_name: &str) -> Vec<String> {
        if self.name == category_name {
            return vec![self.name.clone()];
        }
        for child in &self.children {
            let child_trace = child.get_trace(category_name);
            if !child_trace.is_empty() {
                let mut trace = vec![self.name.clone()];
                trace.extend(child_trace);
                return trace;
            }
        }
        Vec::new()
    }
}

pub fn flatten_categories(categories: &[Category]) -> Vec<String> {
    categories.iter().flat_map(|c| c.flatten()).collect()
}

pub fn flatten_category_paths(categories: &[Category]) -> Vec<Vec<String>> {
    categories.iter().flat_map(|c| c.flatten_paths(&[])).collect()
}
