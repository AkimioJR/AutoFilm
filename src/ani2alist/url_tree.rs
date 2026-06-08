#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Directory { name: String, children: Vec<Node> },
    File(FileEntry),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub modified: i64,
    pub url: String,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn parse(text: &str) -> Self {
        let lines = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        let (children, _) = parse_nodes(&lines, 0, 0);
        Self { children }
    }

    pub fn upsert_path(&mut self, path: &[String], file: FileEntry) {
        if path.is_empty() {
            upsert_file(&mut self.children, file);
            return;
        }

        upsert_path_in_nodes(&mut self.children, path, file);
    }

    pub fn to_structure(&self) -> String {
        stringify_nodes(&self.children, 0).trim_end().to_string()
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_nodes(lines: &[&str], mut index: usize, level: usize) -> (Vec<Node>, usize) {
    let mut nodes = Vec::new();
    while index < lines.len() {
        let line = lines[index];
        let indent = line.chars().take_while(|char| *char == ' ').count();
        let current_level = indent / 2;
        if current_level < level {
            break;
        }
        if current_level > level {
            index += 1;
            continue;
        }

        let trimmed = line.trim();
        if trimmed.ends_with(':') && !contains_url(trimmed) {
            let name = trimmed.trim_end_matches(':').to_string();
            let (children, new_index) = parse_nodes(lines, index + 1, level + 1);
            nodes.push(Node::Directory { name, children });
            index = new_index;
        } else {
            nodes.push(Node::File(parse_file_line(trimmed)));
            index += 1;
        }
    }
    (nodes, index)
}

fn parse_file_line(line: &str) -> FileEntry {
    let url_index = line
        .find("http://")
        .or_else(|| line.find("https://"))
        .unwrap_or(line.len());
    let url = line[url_index..].to_string();
    let info = line[..url_index].trim_end_matches(':');
    let mut parts = info.split(':');
    let name = parts.next().filter(|value| !value.is_empty());
    let size = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_default();
    let modified = parts
        .next()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or_default();

    FileEntry {
        name: name
            .map(ToString::to_string)
            .unwrap_or_else(|| url.rsplit('/').next().unwrap_or("file").to_string()),
        size,
        modified,
        url,
    }
}

fn upsert_path_in_nodes(nodes: &mut Vec<Node>, path: &[String], file: FileEntry) {
    let name = &path[0];
    if path.len() == 1 {
        let children = directory_children(nodes, name);
        upsert_file(children, file);
        return;
    }

    let children = directory_children(nodes, name);
    upsert_path_in_nodes(children, &path[1..], file);
}

fn directory_children<'a>(nodes: &'a mut Vec<Node>, name: &str) -> &'a mut Vec<Node> {
    if let Some(index) = nodes
        .iter()
        .position(|node| matches!(node, Node::Directory { name: node_name, .. } if node_name == name))
    {
        match &mut nodes[index] {
            Node::Directory { children, .. } => children,
            Node::File(_) => unreachable!(),
        }
    } else {
        nodes.push(Node::Directory {
            name: name.to_string(),
            children: Vec::new(),
        });
        match nodes.last_mut().expect("directory was just inserted") {
            Node::Directory { children, .. } => children,
            Node::File(_) => unreachable!(),
        }
    }
}

fn upsert_file(nodes: &mut Vec<Node>, file: FileEntry) {
    if let Some(index) = nodes
        .iter()
        .position(|node| matches!(node, Node::File(existing) if existing.name == file.name))
    {
        nodes[index] = Node::File(file);
    } else {
        nodes.push(Node::File(file));
    }
}

fn stringify_nodes(nodes: &[Node], indent: usize) -> String {
    let mut output = String::new();
    for node in nodes {
        match node {
            Node::Directory { name, children } => {
                output.push_str(&" ".repeat(indent));
                output.push_str(name);
                output.push_str(":\n");
                output.push_str(&stringify_nodes(children, indent + 2));
            }
            Node::File(file) => {
                output.push_str(&" ".repeat(indent));
                output.push_str(&file.name);
                output.push(':');
                output.push_str(&file.size.to_string());
                output.push(':');
                output.push_str(&file.modified.to_string());
                output.push(':');
                output.push_str(&file.url);
                output.push('\n');
            }
        }
    }
    output
}

fn contains_url(value: &str) -> bool {
    value.contains("http://") || value.contains("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_stringifies_url_tree() {
        let tree = Tree::parse(
            "folder:\n  movie.mp4:123:456:https://example.com/movie.mp4\n  sub:\n    file.ass:7:8:https://example.com/file.ass",
        );

        assert_eq!(
            tree.to_structure(),
            "folder:\n  movie.mp4:123:456:https://example.com/movie.mp4\n  sub:\n    file.ass:7:8:https://example.com/file.ass"
        );
    }

    #[test]
    fn upserts_files_and_creates_directories() {
        let mut tree = Tree::new();
        tree.upsert_path(
            &["2026-4".to_string()],
            FileEntry {
                name: "anime.mp4".to_string(),
                size: 10,
                modified: 20,
                url: "https://example.com/old.mp4".to_string(),
            },
        );
        tree.upsert_path(
            &["2026-4".to_string()],
            FileEntry {
                name: "anime.mp4".to_string(),
                size: 30,
                modified: 40,
                url: "https://example.com/new.mp4".to_string(),
            },
        );

        assert_eq!(
            tree.to_structure(),
            "2026-4:\n  anime.mp4:30:40:https://example.com/new.mp4"
        );
    }
}

