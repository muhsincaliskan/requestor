use super::form_part::FormPart;
use super::literal::{quoted, quoted_lines};
use super::snippet::{Snippet, SnippetBody};

const BASE_IMPORTS: [&str; 4] = [
    "java.net.URI",
    "java.net.http.HttpClient",
    "java.net.http.HttpRequest",
    "java.net.http.HttpResponse",
];
const FILE_IMPORTS: [&str; 1] = ["java.nio.file.Path"];
const FORM_IMPORTS: [&str; 4] = [
    "java.nio.charset.StandardCharsets",
    "java.nio.file.Files",
    "java.nio.file.Path",
    "java.util.List",
];
const RESTRICTED_HEADERS: [&str; 5] = ["Connection", "Content-Length", "Expect", "Host", "Upgrade"];
const NON_ASCII_NOTE: &str =
    "        // HttpClient sends header values as ASCII; other characters arrive as ?.\n";
const CLASS_START: &str =
    "\npublic class Main {\n    public static void main(String[] args) throws Exception {\n";
const LINE_JOIN: &str = " +";
const ARGUMENT_JOIN: &str = ",";
const PART_INDENT: &str = "                ";
const CONTINUATION_INDENT: &str = "                        ";
const MULTIPART_LINE_BREAK: &str = "\r\n";
const CONSTANT_CHARACTER_LIMIT: usize = 16_000;
const RESPONSE_OUTPUT: &str = "                .build();\n        HttpResponse<String> response = HttpClient.newHttpClient()\n                .send(request, HttpResponse.BodyHandlers.ofString());\n        System.out.println(response.statusCode());\n        System.out.println(response.body());\n    }\n}\n";

impl Snippet {
    pub(super) fn java(&self) -> String {
        let mut code = self.java_imports();
        code.push_str(CLASS_START);
        if matches!(self.body, SnippetBody::Form { .. }) {
            code.push_str(&self.java_parts());
        }
        if self.headers.iter().any(|header| !header.value.is_ascii()) {
            code.push_str(NON_ASCII_NOTE);
        }
        code.push_str(&format!(
            "        HttpRequest request = HttpRequest.newBuilder()\n                .uri(URI.create({}))\n",
            quoted(&self.url)
        ));
        for header in self.headers_without(&RESTRICTED_HEADERS) {
            code.push_str(&format!(
                "                .header({}, {})\n",
                quoted(&header.name),
                quoted(&header.value)
            ));
        }
        code.push_str(&format!(
            "                .method({}, HttpRequest.BodyPublishers.{})\n",
            quoted(self.method_name()),
            self.java_publisher()
        ));
        code.push_str(RESPONSE_OUTPUT);
        code
    }

    fn java_imports(&self) -> String {
        let mut imports: Vec<&str> = BASE_IMPORTS.to_vec();
        match self.body {
            SnippetBody::File(_) => imports.extend(FILE_IMPORTS),
            SnippetBody::Form { .. } => imports.extend(FORM_IMPORTS),
            SnippetBody::Empty | SnippetBody::Text(_) => {}
        }
        imports.sort_unstable();
        imports.dedup();
        imports
            .iter()
            .map(|import| format!("import {import};\n"))
            .collect()
    }

    fn java_parts(&self) -> String {
        let parts = self
            .form_parts(MULTIPART_LINE_BREAK)
            .iter()
            .map(FormPart::java_bytes)
            .collect::<Vec<_>>()
            .join(&format!(",\n{PART_INDENT}"));
        format!("        List<byte[]> parts = List.of(\n{PART_INDENT}{parts});\n")
    }

    fn java_publisher(&self) -> String {
        match &self.body {
            SnippetBody::Empty => "noBody()".to_string(),
            SnippetBody::Text(text) => {
                format!("ofString(\n{CONTINUATION_INDENT}{})", java_string(text))
            }
            SnippetBody::File(path) => format!("ofFile(Path.of({}))", quoted(path)),
            SnippetBody::Form { .. } => "ofByteArrays(parts)".to_string(),
        }
    }
}

impl FormPart {
    fn java_bytes(&self) -> String {
        match self {
            FormPart::Text(text) => {
                format!("({}).getBytes(StandardCharsets.UTF_8)", java_string(text))
            }
            FormPart::File(path) => format!("Files.readAllBytes(Path.of({}))", quoted(path)),
        }
    }
}

fn java_string(text: &str) -> String {
    if text.chars().count() <= CONSTANT_CHARACTER_LIMIT {
        return quoted_lines(text, LINE_JOIN, CONTINUATION_INDENT);
    }
    let chunks: Vec<String> = constant_sized_chunks(text)
        .into_iter()
        .map(|chunk| quoted_lines(chunk, LINE_JOIN, CONTINUATION_INDENT))
        .collect();
    format!(
        "String.join(\"\",\n{CONTINUATION_INDENT}{})",
        chunks.join(&format!("{ARGUMENT_JOIN}\n{CONTINUATION_INDENT}"))
    )
}

fn constant_sized_chunks(text: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;
    for (position, (index, _)) in text.char_indices().enumerate() {
        if position > 0 && position % CONSTANT_CHARACTER_LIMIT == 0 {
            chunks.push(&text[start..index]);
            start = index;
        }
    }
    chunks.push(&text[start..]);
    chunks
}
