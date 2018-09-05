extern crate ansi_term;
extern crate git2;
extern crate regex;
extern crate tico;
#[macro_use]
extern crate try_opt;

use ansi_term::Colour::{Cyan, Blue, Red, Green, Purple, Yellow};
use ansi_term::{ANSIStrings, ANSIString};
use git2::*;
use regex::Regex;
use std::env;
use tico::tico;

/* Config START here*/
/* Colors */
const AHEAD_COLOR:     ansi_term::Colour = Green;
const BEHIND_COLOR:    ansi_term::Colour = Red;
const BRANCH_COLOR:    ansi_term::Colour = Blue;
const CHANGED_COLOR:   ansi_term::Colour = Purple;
const CLEAN_COLOR:     ansi_term::Colour = Green;
const DIR_COLOR:       ansi_term::Colour = Cyan;
const STAGED_COLOR:    ansi_term::Colour = Yellow;
const UNTRACKED_COLOR: ansi_term::Colour = Blue;
/* Symbols */
const AHEAD_SYMBOL     :&str = ">";
const BEHIND_SYMBOL    :&str = "<";
const CHANGED_SYMBOL   :&str = "+";
const CLEAN_SYMBOL     :&str = "=";
const PRECOMND         :&str = "λ";
const STAGED_SYMBOL    :&str = "-";
const STATUS_PREFIX    :&str = "(";
const STATUS_SEPARATOR :&str = "|";
const STATUS_SUFFIX    :&str = ")";
const UNTRACKED_SYMBOL :&str = "_";
/* Conifg END here*/

// Sorry for bad documentation
fn main() {
    // DIR stuff
    let c_dir = env::current_dir().unwrap();           // get the current dir
    let sh_dir = shorten_dir(c_dir.to_str().unwrap()); // Sorten it.

    // GIT stuff
    let stat = match Repository::discover(c_dir) { // is this dir contain git repo ?
        Ok(repo) => get_status(&repo),             // Yes -> get information about it
        Err(_e) => None,                           // No -> do noting
    };

    // let mut x: ColoredString = sh_dir.color(DIR_COLOR).bold();
    // print our prompot
    println!("");
    println!("{} {}\n{} ",
             DIR_COLOR.paint(sh_dir),
             stat.unwrap_or("".to_string()),
             Purple.paint(PRECOMND))
}

/**
 * shorten_dir: function to putty the dir before display
 * ## Take:
 *    - &str -> cd: which represent the dir
 * ## Return:
 *    - String:
 * ## ex. shorten_dir(~/Documents/Programming/Rust/Apps/pwr)
 *        return: ~/D/P/R/A/pwr
 */
fn shorten_dir(cd: &str) -> String {
    let dir = match env::home_dir() {
        Some(dir) => Regex::new(dir.to_str().unwrap()).unwrap().replace(cd, "~"),
        _ => return String::from("")
    };

    tico(&dir) // an extern crate, see it's sorce code, it less than 50 and so beauty
}

/**
 * get_status: first it collect info in vector, then it return a String
 *             of these informations
 *
 * NOTE: Here wher most of colour stuff done, also, if any int info = 0
 *       I will not show it in prompt
 * XXX: I think this function need a helper function which take a color
 */
fn get_status(r: &Repository) -> Option<String>{
    let mut out = vec![ANSIString::from(STATUS_PREFIX)];

    let branch = branch_name(&r);
    out.push(BRANCH_COLOR.paint(branch.unwrap_or_default()));

    let(s_ahead, s_behind) = ahead_behind(&r).unwrap_or_default();
    if s_behind > 0 {
        out.push(BEHIND_COLOR.paint(BEHIND_SYMBOL));
        out.push(BEHIND_COLOR.paint(s_behind.to_string()));
    }
    if s_ahead > 0 {
        out.push(AHEAD_COLOR.paint(AHEAD_SYMBOL));
        out.push(AHEAD_COLOR.paint(s_ahead.to_string()));
    }

    out.push(ANSIString::from(STATUS_SEPARATOR));

    let(untracked, staged, changed) = status(&r);
    if staged > 0 {
        out.push(STAGED_COLOR.paint(STAGED_SYMBOL));
        out.push(STAGED_COLOR.paint(staged.to_string()));
    }
    if changed > 0 {
        out.push(CHANGED_COLOR.paint(CHANGED_SYMBOL));
        out.push(CHANGED_COLOR.paint(changed.to_string()));
    }
    if untracked > 0 {
        out.push(UNTRACKED_COLOR.paint(UNTRACKED_SYMBOL));
        out.push(UNTRACKED_COLOR.paint(untracked.to_string()));
    }
    if changed+staged+untracked == 0 { out.push(CLEAN_COLOR.paint(CLEAN_SYMBOL)); }

    out.push(ANSIString::from(STATUS_SUFFIX));
    return Some(ANSIStrings(&out).to_string())
}

/**
 * branch_name: check for any head ref in .git/refs/heads
 *
 * TODO: IDK why, but it seem buggy, head refs not alwase there.
 *       I think this shoud check in .git/HEADS. but these stuff related git2 crate
 */
fn branch_name(r: &Repository) -> Option<String> {
    let head = try_opt!(r.head().ok());
    if let Some(shorthand) = head.shorthand() {
        if shorthand != "HEAD" {
            return Some(shorthand.to_string())
        }
    }

    let object = try_opt!(head.peel(git2::ObjectType::Commit).ok());
    let short_id = try_opt!(object.short_id().ok());

    Some(format!(":{}", short_id.iter().map(|ch| *ch as char).collect::<String>()))
}

/**
 * ahead_behind
 */
fn ahead_behind(r: &Repository) -> Option<(usize, usize)>{
    let mut s_ahead  : usize = 0;
    let mut s_behind : usize = 0;
    let head = try_opt!(r.head().ok());
    let h_oid = head.target();
    let raw_branch = Branch::wrap(head);
    let remote_oid = match raw_branch.upstream() {
        Ok(raw_upstream) => raw_upstream.into_reference().target(),
        _ => None
    };

    if let (Some(oid), Some(roid)) = (h_oid, remote_oid) {
        if let Ok((ahead, behind)) = r.graph_ahead_behind(oid, roid) {
            s_ahead = ahead;
            s_behind = behind;
        }
    }
    return Some((s_ahead, s_behind))
}

/**
 * status: check for
 *         - Changes
 *         - Untracked files
 *         - Staged
 *
 */
fn status(r: &Repository) -> (usize, usize, usize){
    let mut untracked: usize = 0;
    let mut staged: usize = 0;
    let mut changed: usize = 0;
    if let Ok(raw_statuses) = r.statuses(None) {
        for raw_status in raw_statuses.iter().map(|e| e.status()) {
            if raw_status.intersects(git2::Status::WT_NEW) {
                untracked += 1;
            }
            if raw_status.intersects(git2::Status::INDEX_NEW | git2::Status::INDEX_MODIFIED |
                                     git2::Status::INDEX_DELETED | git2::Status::INDEX_RENAMED |
                                     git2::Status::INDEX_TYPECHANGE) {
                staged += 1;
            }
            if raw_status.intersects(git2::Status::WT_MODIFIED | git2::Status::WT_DELETED |
                                     git2::Status::WT_TYPECHANGE | git2::Status::WT_RENAMED) {
                changed += 1;
            }
        }
    }
    return (untracked, staged, changed)
}
