use anchor_lang::declare_program;
use anchor_litesvm::{
    AnchorContext, AnchorLiteSVM, Pubkey, Signer, TestHelpers, TransactionResult,
};

declare_program!(voting);

use self::voting::accounts::{CandidateAccount, PollAccount};
use self::voting::client::{accounts, args};

const PROGRAM_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/deploy/voting.so"
));

// ================= SETUP =================

fn setup() -> AnchorContext {
    use anchor_lang::solana_program::clock::Clock;

    let mut ctx = AnchorLiteSVM::build_with_program(self::voting::ID, PROGRAM_BYTES);

    let clock = Clock {
        slot: 1000,
        epoch_start_timestamp: 0,
        epoch: 1,
        leader_schedule_epoch: 1,
        unix_timestamp: 1000,
    };

    ctx.svm.set_sysvar(&clock);
    ctx
}

// ================= PDA HELPERS =================

fn poll_pda(poll_id: u64) -> Pubkey {
    Pubkey::find_program_address(&[b"poll", &poll_id.to_le_bytes()], &self::voting::ID).0
}

fn candidate_pda(poll_id: u64, candidate: &str) -> Pubkey {
    Pubkey::find_program_address(
        &[&poll_id.to_le_bytes(), candidate.as_bytes()],
        &self::voting::ID,
    )
    .0
}

// ================= INSTRUCTIONS =================

fn init_poll(
    ctx: &mut AnchorContext,
    signer: &anchor_litesvm::Keypair,
    poll_id: u64,
    start: u64,
    end: u64,
    name: &str,
    description: &str,
) {
    let ix = ctx
        .program()
        .accounts(accounts::InitPoll {
            signer: signer.pubkey(),
            poll_account: poll_pda(poll_id),
            system_program: anchor_lang::system_program::ID,
        })
        .args(args::InitPoll {
            _poll_id: poll_id,
            start,
            end,
            name: name.to_string(),
            description: description.to_string(),
        })
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[signer])
        .unwrap()
        .assert_success();
}

fn init_candidate(
    ctx: &mut AnchorContext,
    signer: &anchor_litesvm::Keypair,
    poll_id: u64,
    candidate: &str,
) {
    let ix = ctx
        .program()
        .accounts(accounts::InitializeCandidate {
            signer: signer.pubkey(),
            poll_account: poll_pda(poll_id),
            candidate_account: candidate_pda(poll_id, candidate),
            system_program: anchor_lang::system_program::ID,
        })
        .args(args::InitializeCandidate {
            _poll_id: poll_id,
            candidate: candidate.to_string(),
        })
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[signer])
        .unwrap()
        .assert_success();
}

fn vote(
    ctx: &mut AnchorContext,
    signer: &anchor_litesvm::Keypair,
    poll_id: u64,
    candidate: &str,
) -> TransactionResult {
    let ix = ctx
        .program()
        .accounts(accounts::Vote {
            signer: signer.pubkey(),
            poll_account: poll_pda(poll_id),
            candidate_account: candidate_pda(poll_id, candidate),
        })
        .args(args::Vote {
            _poll_id: poll_id,
            _candidate: candidate.to_string(),
        })
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[signer]).unwrap()
}

// ================= TESTS =================

#[test]
fn test_poll_creation() {
    let mut ctx = setup();
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    init_poll(&mut ctx, &user, 1, 0, u64::MAX, "Test Poll", "Description");

    let poll: PollAccount = ctx.get_account(&poll_pda(1)).unwrap();

    assert_eq!(poll.poll_name, "Test Poll");
    assert_eq!(poll.poll_description, "Description");
    assert_eq!(poll.poll_option_index, 0);
}

#[test]
fn test_candidate_creation() {
    let mut ctx = setup();
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    init_poll(&mut ctx, &user, 1, 0, u64::MAX, "Poll", "Desc");

    init_candidate(&mut ctx, &user, 1, "Alice");
    init_candidate(&mut ctx, &user, 1, "Bob");

    let poll: PollAccount = ctx.get_account(&poll_pda(1)).unwrap();
    let alice: CandidateAccount = ctx.get_account(&candidate_pda(1, "Alice")).unwrap();

    assert_eq!(poll.poll_option_index, 2);
    assert_eq!(alice.candidate_votes, 0);
}

#[test]
fn test_vote_success() {
    let mut ctx = setup();
    let admin = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let voter = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    init_poll(&mut ctx, &admin, 1, 0, 2000, "Poll", "Desc");
    init_candidate(&mut ctx, &admin, 1, "Alice");

    vote(&mut ctx, &voter, 1, "Alice").assert_success();

    let alice: CandidateAccount = ctx.get_account(&candidate_pda(1, "Alice")).unwrap();

    assert_eq!(alice.candidate_votes, 1);
}

#[test]
fn test_vote_before_start() {
    let mut ctx = setup();
    let admin = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let voter = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    init_poll(&mut ctx, &admin, 1, 1500, 2000, "Poll", "Desc");
    init_candidate(&mut ctx, &admin, 1, "Alice");

    vote(&mut ctx, &voter, 1, "Alice")
        .assert_failure()
        .assert_anchor_error("VotingNotStarted");
}

#[test]
fn test_vote_after_end() {
    let mut ctx = setup();
    let admin = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let voter = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    init_poll(&mut ctx, &admin, 1, 0, 500, "Poll", "Desc");
    init_candidate(&mut ctx, &admin, 1, "Alice");

    vote(&mut ctx, &voter, 1, "Alice")
        .assert_failure()
        .assert_anchor_error("VotingEnded");
}

#[test]
fn debug_program_id() {
    println!("PROGRAM ID: {}", self::voting::ID);
}
