pub fn parse_path(cur_path: &str, next_path: &str) -> String {
    let path_parts: Vec<&str> = next_path.split("/").collect();
    let cur_parts: Vec<&str> = cur_path.split("/").collect();

    // Determine if it's relative
    let is_relative = path_parts.contains(&".") || path_parts.contains(&"..");
    if !is_relative {
        return String::from(next_path);
    }

    // Initialize the new path
    let mut new_path: Vec<&str> = Vec::new();
    if let Some(part) = path_parts.first() {
        if part == &"." || part == &".." {
            new_path.extend(cur_parts.iter());
        }
    }

    // Go through the path parts
    for part in &path_parts {
        if part == &".." {
            new_path.pop();
            new_path.pop();
        } else if part == &"." {
            new_path.pop();
        } else {
            new_path.push(part);
        }
    }

    return new_path.join("/");
}
