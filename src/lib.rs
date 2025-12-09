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
    let repos_root = env.call("straight--repos-dir", [])?;
    let repos_root_str = repos_root.into_rust::<String>()?;

    let (msg_tx, msg_rx) = crossbeam_channel::unbounded::<String>();

    let pull_thread = std::thread::spawn(move || {
        let repos = fs::read_dir(&repos_root_str)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|e| {
                e.is_dir() && (e.file_name().unwrap() != "." || e.file_name().unwrap() != "..")
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
                6
            }
        };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads_num)
            .build()
            .unwrap();
        pool.install(|| {
            repos.into_par_iter().for_each(|repo_dir| {
                let git_repo = match Repository::open(repo_dir.display().to_string()) {
                    Ok(repo) => repo,
                    Err(err) => {
                        msg_tx
                            .send(format!(
                                "failed to open repo {:?} with error: {:?}",
                                repo_dir.display(),
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
                            repo_dir.display(),
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
    env.message("straight-utils-module-pull-all is finished!")?;

    Ok(())
}
