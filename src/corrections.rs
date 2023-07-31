use std::collections::HashMap;

use rule_parser::parse_rules;

use crate::shell::{command_output, PRIVILEGE_LIST};
use crate::style::highlight_difference;

pub fn correct_command(shell: &str, last_command: &str) -> Option<String> {
	let command_output = command_output(shell, last_command);

	let split_command = last_command.split_whitespace().collect::<Vec<&str>>();
	let command = match PRIVILEGE_LIST.contains(&split_command[0]) {
		true => split_command.get(1).expect("No command found."),
		false => split_command.first().expect("No command found."),
	};

	if !PRIVILEGE_LIST.contains(command) {
		let suggest = match_pattern("privilege", &command_output);
		if let Some(suggest) = suggest {
			let suggest = eval_suggest(&suggest, last_command);
			return Some(suggest);
		}
	}
	let suggest = match_pattern(command, &command_output);
	if let Some(suggest) = suggest {
		let suggest = eval_suggest(&suggest, last_command);
		if PRIVILEGE_LIST.contains(command) {
			return Some(format!("{} {}", split_command[0], suggest));
		}
		return Some(suggest);
	}
	None
}

fn match_pattern(command: &str, error_msg: &str) -> Option<String> {
	let rules = parse_rules!("rules");
	if rules.contains_key(command) {
		let suggest = rules.get(command).unwrap();
		for (pattern, suggest) in suggest {
			for pattern in pattern {
				if error_msg.contains(pattern) {
					for suggest in suggest {
						if let Some(suggest) = check_suggest(suggest) {
							return Some(suggest);
						}
					}
				}
			}
		}
		None
	} else {
		None
	}
}

fn check_suggest(suggest: &str) -> Option<String> {
	if !suggest.starts_with('#') {
		return Some(suggest.to_owned());
	}
	let lines = suggest.lines().collect::<Vec<&str>>();
	let conditions = lines.first().unwrap();
	let conditions = conditions.trim_matches(|c| c == '#' || c == '[' || c == ']');
	let conditions = conditions.split(',').collect::<Vec<&str>>();
	for condition in conditions {
		let condition = condition.trim();
		let (condition, arg) = condition.split_once('(').unwrap();
		let arg = arg.trim_matches(|c| c == '(' || c == ')');

		if eval_condition(condition, arg) == false {
			return None;
		}
	}
	Some(lines[1..].join("\n"))
}

fn eval_condition(condition: &str, arg: &str) -> bool {
	match condition {
		"executable" => {
			let output = std::process::Command::new("which")
				.arg(arg)
				.output()
				.expect("failed to execute process");
			output.status.success()
		}
		_ => false,
	}
}

fn eval_suggest(suggest: &str, last_command: &str) -> String {
	let mut suggest = suggest.to_owned();
	if suggest.contains("{{command}}") {
		suggest = suggest.replace("{{command}}", last_command);
	}
	while suggest.contains("{{command") {
		let placeholder_start = "{{command";
		let placeholder_end = "}}";
		let placeholder = suggest.find(placeholder_start).unwrap()
			..suggest.find(placeholder_end).unwrap() + placeholder_end.len();

		let range = suggest[placeholder.to_owned()].trim_matches(|c| c == '[' || c == ']');
		if let Some((start, end)) = range.split_once(':') {
			let start = match start {
				"" => 0,
				_ => start.parse::<usize>().unwrap(),
			};
			let end = match end {
				"" => last_command.split_whitespace().count(),
				_ => end.parse::<usize>().unwrap(),
			};
			let split_command = last_command.split_whitespace().collect::<Vec<&str>>();
			let command = split_command[start..end].join(" ");
			suggest = suggest.replace(&suggest[placeholder], &command);
		} else {
			let range = range.parse::<usize>().unwrap();
			let split_command = last_command.split_whitespace().collect::<Vec<&str>>();
			let command = split_command[range].to_owned();
			suggest = suggest.replace(&suggest[placeholder], &command);
		}
	}

	suggest
}

pub fn confirm_correction(shell: &str, command: &str, last_command: &str) {
	println!(
		"Did you mean {}?",
		highlight_difference(command, last_command)
	);
	println!("Press enter to execute the corrected command. Or press Ctrl+C to exit.");
	std::io::stdin().read_line(&mut String::new()).unwrap();

	for p in PRIVILEGE_LIST {
		if command.starts_with(p) {
			let command = command.replace(p, "");
			std::process::Command::new(p.trim())
				.arg(shell)
				.arg("-c")
				.arg(command)
				.spawn()
				.expect("failed to execute process")
				.wait()
				.expect("failed to wait on process");
			return;
		}
	}

	std::process::Command::new(shell)
		.arg("-c")
		.arg(command)
		.spawn()
		.expect("failed to execute process")
		.wait()
		.expect("failed to wait on process");
}
