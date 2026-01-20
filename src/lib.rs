use std::fs;

use emacs::{Env, Result, defun};
use git2::Repository;
use rayon::prelude::*;

emacs::plugin_is_GPL_compatible!();

#[emacs::module(name(fn))]
fn straight_utils_module(env: &Env) -> Result<()> {
    env.message("straight-utils-module is loaded!")?;
    Ok(())
}

#[defun]
fn pull_all(env: &Env) -> Result<()> {
    let repos_root_path = env.call("straight--repos-dir", [])?;
    let repos_root_path_str = repos_root_path.into_rust::<String>()?;

    let (msg_tx, msg_rx) = crossbeam_channel::unbounded::<String>();

    let pull_thread = std::thread::spawn(move || {
        let repo_paths = fs::read_dir(&repos_root_path_str)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|entry| {
                entry.is_dir()
                    && (entry.file_name().unwrap() != "." || entry.file_name().unwrap() != "..")
            })
            .collect::<Vec<_>>();

        let threads_num: usize = match std::env::var("RAYON_NUM_THREADS") {
            Ok(val) => val.parse().unwrap(),
            Err(err) => {
                if err != std::env::VarError::NotPresent {
                    msg_tx
                        .send(format!("parse RAYON_NUM_THREADS failed: {:?}", err))
                        .unwrap();
                }
                num_cpus::get()
            }
        };

        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads_num)
            .build()
            .unwrap();
        thread_pool.install(|| {
            repo_paths.into_par_iter().for_each(|repo_path| {
                let git_repo = match Repository::open(repo_path.display().to_string()) {
                    Ok(repo) => repo,
                    Err(err) => {
                        msg_tx
                            .send(format!(
                                "failed to open repo {:?} with error: {:?}",
                                repo_path.display(),
                                err,
                            ))
                            .unwrap();
                        return;
                    }
                };

                let mut remote = git_repo.find_remote("origin").unwrap();
                let result = remote.fetch(&["main", "master"], None, None);
                if result.is_err() {
                    msg_tx
                        .send(format!(
                            "failed to update repo {:?} with error: {:?}",
                            repo_path.display(),
                            result.unwrap(),
                        ))
                        .unwrap();
                    return;
                }

                let fetch_head = git_repo.find_reference("FETCH_HEAD").unwrap();
                let fetch_commit = git_repo.reference_to_annotated_commit(&fetch_head).unwrap();
                let fetch_oid = fetch_commit.id();

                let obj = git_repo.find_object(fetch_oid, None).unwrap();
                git_repo.reset(&obj, git2::ResetType::Hard, None).unwrap();
            });
        });
    });

    for msg in msg_rx {
        env.message(msg)?;
    }

    pull_thread.join().unwrap();
    env.message("all packages are updated!")?;

    Ok(())
}
