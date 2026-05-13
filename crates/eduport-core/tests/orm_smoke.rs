//! End-to-end smoke test for the vaultdb-orm derive on every eduport
//! entity variant. Drops markdown files at a temp vault root, then
//! runs `Query::<T>::new(&vault).fetch()` for each type and asserts
//! the discriminator did its job — only matching-typed records come
//! back.

use std::fs;

use eduport_core::entity::{Application, Document, Email, Lab, Note, Person, Program, University};
use tempfile::TempDir;
use vaultdb_core::Vault;
use vaultdb_orm::Query;

fn write(dir: &TempDir, stem: &str, frontmatter: &str) {
    fs::write(
        dir.path().join(format!("{stem}.md")),
        format!("---\n{frontmatter}\n---\nbody\n"),
    )
    .unwrap();
}

fn setup() -> (TempDir, Vault) {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join(".obsidian")).unwrap();

    // One entity per type, plus one bare note that shouldn't match
    // anything but Note.
    write(
        &dir,
        "stanford",
        "name: Stanford\ntags:\n  - eduport-type/university\ncountry: USA",
    );
    write(
        &dir,
        "csail",
        "name: CSAIL\ntags:\n  - eduport-type/lab\nfocus: AI",
    );
    write(&dir, "alice", "name: Alice\ntags:\n  - eduport-type/person");
    write(
        &dir,
        "phd-cs",
        "name: CS PhD\ntags:\n  - eduport-type/program\nlevel: phd",
    );
    write(
        &dir,
        "stanford-cs-2026",
        "name: Stanford CS 2026\ntags:\n  - eduport-type/application\nprogram: \"[[Stanford CS PhD]]\"\nstatus: drafting",
    );
    write(
        &dir,
        "transcript",
        "name: Transcript\ntags:\n  - eduport-type/document\ntitle: My transcript",
    );
    write(
        &dir,
        "msg-001",
        "name: msg-001\ntags:\n  - eduport-type/email\ndirection: inbound\ndate: \"2026-05-10\"\nsubject: Hi\nfrom: a@b.c",
    );
    write(
        &dir,
        "scratch",
        "name: Scratch\ntags:\n  - eduport-type/note",
    );

    let vault = Vault::with_root(dir.path().to_path_buf());
    (dir, vault)
}

#[test]
fn typed_query_pins_to_university_via_discriminator() {
    let (_dir, vault) = setup();
    let unis: Vec<University> = Query::<University>::new(&vault).fetch().unwrap();
    assert_eq!(unis.len(), 1);
    assert_eq!(unis[0].name, "Stanford");
    assert_eq!(unis[0].country, "USA");
}

#[test]
fn typed_query_pins_to_lab() {
    let (_dir, vault) = setup();
    let labs: Vec<Lab> = Query::<Lab>::new(&vault).fetch().unwrap();
    assert_eq!(labs.len(), 1);
    assert_eq!(labs[0].name, "CSAIL");
}

#[test]
fn typed_query_pins_to_person() {
    let (_dir, vault) = setup();
    let people: Vec<Person> = Query::<Person>::new(&vault).fetch().unwrap();
    assert_eq!(people.len(), 1);
    assert_eq!(people[0].name, "Alice");
}

#[test]
fn typed_query_pins_to_program() {
    let (_dir, vault) = setup();
    let programs: Vec<Program> = Query::<Program>::new(&vault).fetch().unwrap();
    assert_eq!(programs.len(), 1);
    assert_eq!(programs[0].name, "CS PhD");
}

#[test]
fn typed_query_pins_to_application() {
    let (_dir, vault) = setup();
    let apps: Vec<Application> = Query::<Application>::new(&vault).fetch().unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].name, "Stanford CS 2026");
    assert_eq!(apps[0].program.target, "Stanford CS PhD");
}

#[test]
fn typed_query_pins_to_document() {
    let (_dir, vault) = setup();
    let docs: Vec<Document> = Query::<Document>::new(&vault).fetch().unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].name, "Transcript");
}

#[test]
fn typed_query_pins_to_email() {
    let (_dir, vault) = setup();
    let emails: Vec<Email> = Query::<Email>::new(&vault).fetch().unwrap();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].name, "msg-001");
    assert_eq!(emails[0].from, "a@b.c");
}

#[test]
fn typed_query_pins_to_note() {
    let (_dir, vault) = setup();
    let notes: Vec<Note> = Query::<Note>::new(&vault).fetch().unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].name, "Scratch");
}

#[test]
fn typed_filter_combines_with_discriminator() {
    let (_dir, vault) = setup();
    // Stricter filter: Stanford only.
    let unis: Vec<University> = Query::<University>::new(&vault)
        .filter(University::country().eq("USA"))
        .fetch()
        .unwrap();
    assert_eq!(unis.len(), 1);

    let other: Vec<University> = Query::<University>::new(&vault)
        .filter(University::country().eq("UK"))
        .fetch()
        .unwrap();
    assert!(other.is_empty());
}
