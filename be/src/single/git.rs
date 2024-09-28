use std::{
    io::{self, Write},
    process::Command,
};

use git2::Repository;
use serde::Serialize;

use crate::{config, Assignment, Error, Result, Submission};

fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
) -> core::result::Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut fo = git2::FetchOptions::new();
    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(git2::AutotagOption::All);
    remote.fetch(refs, Some(&mut fo), None)?;

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    repo.reference_to_annotated_commit(&fetch_head)
}

fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> core::result::Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    println!("{}", msg);
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
            // I suspect we should be adding some logic to handle dirty working directory states
            // but this is just an example so maybe not.
            .force(),
    ))?;
    Ok(())
}

fn normal_merge(
    repo: &Repository,
    local: &git2::AnnotatedCommit,
    remote: &git2::AnnotatedCommit,
) -> core::result::Result<(), git2::Error> {
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    if idx.has_conflicts() {
        println!("Merge conflicts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }
    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    // now create the merge commit
    let msg = format!("Merge: {} into {}", remote.id(), local.id());
    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    // Do our merge commit and set current branch head to that commit.
    let _merge_commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &msg,
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;
    // Set working tree to match head.
    repo.checkout_head(None)?;
    Ok(())
}

fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> core::result::Result<(), git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appropriate merge
    if analysis.0.is_fast_forward() {
        println!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(&repo, &head_commit, &fetch_commit)?;
    } else {
        // println!("Nothing to do...");
    }
    Ok(())
}

fn do_pull(path: &String) -> core::result::Result<(), git2::Error> {
    // the code used here is extracted directly from the pull.rs example on crate
    let repo = Repository::open(path)?;
    let mut remote = repo.find_remote("origin")?;
    let fetch_commit = do_fetch(&repo, &["main"], &mut remote)?;
    do_merge(&repo, "main", fetch_commit)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub enum AssignmentStatus {
    NotSubmitted,
    Submitted,
    Late,
}
#[derive(Debug, Serialize)]
pub struct AssignmentReport {
    pub name: String,
    pub status: AssignmentStatus,
    pub content: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Report {
    pub id: i64,
    pub assignments: Vec<AssignmentReport>,
}

pub fn report(assigments: &[Assignment], submissions: &[Submission]) -> Result<Vec<Report>> {
    let mut all_students_report: Vec<Report> = vec![];
    for submission in submissions {
        println!("\n\nprocessing: {submission:?}");
        // pull or clone the repo
        // FIXME: this is a good place to setup a logline as there is
        // a lot of heavy lifting and not all of that is
        // reflected in the output
        let path = format!("{}/{}", config().downloads, submission.id);
        if let Err(pull_error) = do_pull(&path) {
            println!("could not pull repo: {pull_error:?}");
            if let Err(clone_error) = Repository::clone(&submission.link, &path) {
                //FIXME: looks like a file is still created here
                println!("could not clone repo: {clone_error:?}");
                continue;
            }
        }

        let mut per_student_report: Vec<AssignmentReport> = vec![];
        for assignment in assigments {
            // check if the assignment was submitted within deadline
            let output = Command::new("git")
                .arg("log")
                .arg("-1")
                .arg("--date=unix")
                .arg("--format=%cd")
                .arg("--")
                .arg(assignment.filepath.as_str())
                .current_dir(&path)
                .output()?;
            // TODO: might be nice to set up some tracing here as
            // this part is doing all the heavy lifting
            let s = std::str::from_utf8(output.stdout.as_ref())?.trim();
            let submitted_timestamp: i64 = s.parse().unwrap_or(0);
            let status = match submitted_timestamp {
                v if v > assignment.deadline.timestamp() => AssignmentStatus::Late,
                0 => AssignmentStatus::NotSubmitted,
                _ => AssignmentStatus::Submitted,
            };
            let assignment_report = AssignmentReport {
                name: assignment.name.clone(),
                status,
                content: None, // FIXME: eventually this should have the pdf content
            };
            per_student_report.push(assignment_report);
        }
        all_students_report.push(Report {
            id: submission.id,
            assignments: per_student_report,
        });
    }
    Ok(all_students_report)
}
