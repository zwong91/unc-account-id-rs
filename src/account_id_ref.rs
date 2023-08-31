use std::borrow::Cow;

use crate::{validation::validate, AccountId, ParseAccountError};

/// Account identifier. This is the human readable UTF-8 string which is used internally to index
/// accounts on the network and their respective state.
///
/// This is the "referenced" version of the account ID. It is to [`AccountId`] what [`str`] is to [`String`],
/// and works quite similarly to [`Path`]. Like with [`str`] and [`Path`], you
/// can't have a value of type `AccountIdRef`, but you can have a reference like `&AccountIdRef` or
/// `&mut AccountIdRef`.
///
/// This type supports zero-copy deserialization offered by [`serde`](https://docs.rs/serde/), but cannot
/// do the same for [`borsh`](https://docs.rs/borsh/) since the latter does not support zero-copy.
///
/// # Examples
/// ```
/// use near_account_id::{AccountId, AccountIdRef};
/// use std::convert::{TryFrom, TryInto};
///
/// // Construction
/// let alice = AccountIdRef::new("alice.near").unwrap();
/// assert!(AccountIdRef::new("invalid.").is_err());
///
/// // Initialize without validating
/// let alice_unchecked = AccountIdRef::new_unchecked("alice.near");
/// assert_eq!(alice, alice_unchecked);
/// ```
///
/// [`FromStr`]: std::str::FromStr
/// [`Path`]: std::path::Path
#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[cfg_attr(feature = "abi", derive(schemars::JsonSchema, BorshSchema))]
pub struct AccountIdRef(pub(crate) str);

impl AccountIdRef {
    /// Shortest valid length for a NEAR Account ID.
    pub const MIN_LEN: usize = crate::validation::MIN_LEN;
    /// Longest valid length for a NEAR Account ID.
    pub const MAX_LEN: usize = crate::validation::MAX_LEN;

    /// Construct a [`&AccountIdRef`](AccountIdRef) from a string reference.
    ///
    /// This constructor validates the provided ID, and will produce an error when validation fails.
    pub fn new<S: AsRef<str> + ?Sized>(id: &S) -> Result<&Self, ParseAccountError> {
        let id = id.as_ref();
        validate(id)?;

        // Safety:
        // - a newtype struct is guaranteed to have the same memory layout as its only field
        // - the borrow checker will enforce its rules appropriately on the resulting reference
        Ok(unsafe { &*(id as *const str as *const Self) })
    }

    /// Construct a [`&AccountIdRef`](AccountIdRef) from a string reference without validating the address.
    /// It is the responsibility of the caller to ensure the account ID is valid.
    ///
    /// For more information, read: <https://docs.near.org/docs/concepts/account#account-id-rules>
    pub fn new_unchecked<S: AsRef<str> + ?Sized>(id: &S) -> &Self {
        let id = id.as_ref();
        debug_assert!(validate(id).is_ok());

        // Safety: see `AccountId::new`
        unsafe { &*(id as *const str as *const Self) }
    }

    /// Returns a reference to the account ID bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Returns a string slice of the entire Account ID.
    ///
    /// ## Examples
    ///
    /// ```
    /// use near_account_id::AccountIdRef;
    ///
    /// let carol = AccountIdRef::new("carol.near").unwrap();
    /// assert_eq!("carol.near", carol.as_str());
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns `true` if the account ID is a top-level NEAR Account ID.
    ///
    /// See [Top-level Accounts](https://docs.near.org/docs/concepts/account#top-level-accounts).
    ///
    /// ## Examples
    ///
    /// ```
    /// use near_account_id::AccountIdRef;
    ///
    /// let near_tla = AccountIdRef::new("near").unwrap();
    /// assert!(near_tla.is_top_level());
    ///
    /// // "alice.near" is a sub account of "near" account
    /// let alice = AccountIdRef::new("alice.near").unwrap();
    /// assert!(!alice.is_top_level());
    /// ```
    pub fn is_top_level(&self) -> bool {
        !self.is_system() && !self.0.contains('.')
    }

    /// Returns `true` if the `AccountId` is a direct sub-account of the provided parent account.
    ///
    /// See [Subaccounts](https://docs.near.org/docs/concepts/account#subaccounts).
    ///
    /// ## Examples
    ///
    /// ```
    /// use near_account_id::AccountId;
    ///
    /// let near_tla: AccountId = "near".parse().unwrap();
    /// assert!(near_tla.is_top_level());
    ///
    /// let alice: AccountId = "alice.near".parse().unwrap();
    /// assert!(alice.is_sub_account_of(&near_tla));
    ///
    /// let alice_app: AccountId = "app.alice.near".parse().unwrap();
    ///
    /// // While app.alice.near is a sub account of alice.near,
    /// // app.alice.near is not a sub account of near
    /// assert!(alice_app.is_sub_account_of(&alice));
    /// assert!(!alice_app.is_sub_account_of(&near_tla));
    /// ```
    pub fn is_sub_account_of(&self, parent: &AccountIdRef) -> bool {
        self.0
            .strip_suffix(parent.as_str())
            .and_then(|s| s.strip_suffix('.'))
            .map_or(false, |s| !s.contains('.'))
    }

    /// Returns `true` if the `AccountId` is a 64 characters long hexadecimal.
    ///
    /// See [Implicit-Accounts](https://docs.near.org/docs/concepts/account#implicit-accounts).
    ///
    /// ## Examples
    ///
    /// ```
    /// use near_account_id::AccountId;
    ///
    /// let alice: AccountId = "alice.near".parse().unwrap();
    /// assert!(!alice.is_implicit());
    ///
    /// let rando = "98793cd91a3f870fb126f66285808c7e094afcfc4eda8a970f6648cdf0dbd6de"
    ///     .parse::<AccountId>()
    ///     .unwrap();
    /// assert!(rando.is_implicit());
    /// ```
    pub fn is_implicit(&self) -> bool {
        self.0.len() == 64
            && self
                .as_bytes()
                .iter()
                .all(|b| matches!(b, b'a'..=b'f' | b'0'..=b'9'))
    }

    /// Returns `true` if this `AccountId` is the system account.
    ///
    /// See [System account](https://nomicon.io/DataStructures/Account.html?highlight=system#system-account).
    ///
    /// ## Examples
    ///
    /// ```
    /// use near_account_id::AccountId;
    ///
    /// let alice: AccountId = "alice.near".parse().unwrap();
    /// assert!(!alice.is_system());
    ///
    /// let system: AccountId = "system".parse().unwrap();
    /// assert!(system.is_system());
    /// ```
    pub fn is_system(&self) -> bool {
        self == "system"
    }
}

impl std::fmt::Display for AccountIdRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl ToOwned for AccountIdRef {
    type Owned = AccountId;

    fn to_owned(&self) -> Self::Owned {
        AccountId(self.0.into())
    }
}

impl<'a> From<&'a AccountIdRef> for AccountId {
    fn from(id: &'a AccountIdRef) -> Self {
        id.to_owned()
    }
}

impl<'s> TryFrom<&'s str> for &'s AccountIdRef {
    type Error = ParseAccountError;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        AccountIdRef::new(value)
    }
}

impl AsRef<str> for AccountIdRef {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<AccountIdRef> for String {
    fn eq(&self, other: &AccountIdRef) -> bool {
        self == &other.0
    }
}

impl PartialEq<String> for AccountIdRef {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<AccountIdRef> for str {
    fn eq(&self, other: &AccountIdRef) -> bool {
        self == &other.0
    }
}

impl PartialEq<str> for AccountIdRef {
    fn eq(&self, other: &str) -> bool {
        &self.0 == other
    }
}

impl<'a> PartialEq<AccountIdRef> for &'a str {
    fn eq(&self, other: &AccountIdRef) -> bool {
        *self == &other.0
    }
}

impl<'a> PartialEq<&'a str> for AccountIdRef {
    fn eq(&self, other: &&'a str) -> bool {
        &self.0 == *other
    }
}

impl<'a> PartialEq<&'a AccountIdRef> for str {
    fn eq(&self, other: &&'a AccountIdRef) -> bool {
        self == &other.0
    }
}

impl<'a> PartialEq<str> for &'a AccountIdRef {
    fn eq(&self, other: &str) -> bool {
        &self.0 == other
    }
}

impl<'a> PartialEq<&'a AccountIdRef> for String {
    fn eq(&self, other: &&'a AccountIdRef) -> bool {
        self == &other.0
    }
}

impl<'a> PartialEq<String> for &'a AccountIdRef {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialOrd<AccountIdRef> for String {
    fn partial_cmp(&self, other: &AccountIdRef) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(&other.0)
    }
}

impl PartialOrd<String> for AccountIdRef {
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other.as_str())
    }
}

impl PartialOrd<AccountIdRef> for str {
    fn partial_cmp(&self, other: &AccountIdRef) -> Option<std::cmp::Ordering> {
        self.partial_cmp(other.as_str())
    }
}

impl PartialOrd<str> for AccountIdRef {
    fn partial_cmp(&self, other: &str) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(other)
    }
}

impl<'a> PartialOrd<AccountIdRef> for &'a str {
    fn partial_cmp(&self, other: &AccountIdRef) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.as_str())
    }
}

impl<'a> PartialOrd<&'a str> for AccountIdRef {
    fn partial_cmp(&self, other: &&'a str) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(*other)
    }
}

impl<'a> PartialOrd<&'a AccountIdRef> for String {
    fn partial_cmp(&self, other: &&'a AccountIdRef) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(&other.0)
    }
}

impl<'a> PartialOrd<String> for &'a AccountIdRef {
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other.as_str())
    }
}

impl<'a> PartialOrd<&'a AccountIdRef> for str {
    fn partial_cmp(&self, other: &&'a AccountIdRef) -> Option<std::cmp::Ordering> {
        self.partial_cmp(other.as_str())
    }
}

impl<'a> PartialOrd<str> for &'a AccountIdRef {
    fn partial_cmp(&self, other: &str) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(other)
    }
}

impl<'a> From<&'a AccountIdRef> for Cow<'a, AccountIdRef> {
    fn from(value: &'a AccountIdRef) -> Self {
        Cow::Borrowed(value)
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for &'a AccountIdRef {
    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (crate::validation::MIN_LEN, Some(crate::validation::MAX_LEN))
    }

    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut s = u.arbitrary::<&str>()?;

        loop {
            match AccountIdRef::new(s) {
                Ok(account_id) => break Ok(account_id),
                Err(ParseAccountError {
                    char: Some((idx, _)),
                    ..
                }) => {
                    s = &s[..idx];
                    continue;
                }
                _ => break Err(arbitrary::Error::IncorrectFormat),
            }
        }
    }

    fn arbitrary_take_rest(u: arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let s = <&str as arbitrary::Arbitrary>::arbitrary_take_rest(u)?;
        AccountIdRef::new(s).map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

#[cfg(test)]
mod tests {
    use crate::ParseErrorKind;

    use super::*;

    #[test]
    fn test_err_kind_classification() {
        let id = AccountIdRef::new("ErinMoriarty.near");
        debug_assert!(
            matches!(
                id,
                Err(ParseAccountError {
                    kind: ParseErrorKind::InvalidChar,
                    char: Some((0, 'E'))
                })
            ),
            "{:?}",
            id
        );

        let id = AccountIdRef::new("-KarlUrban.near");
        debug_assert!(
            matches!(
                id,
                Err(ParseAccountError {
                    kind: ParseErrorKind::RedundantSeparator,
                    char: Some((0, '-'))
                })
            ),
            "{:?}",
            id
        );

        let id = AccountIdRef::new("anthonystarr.");
        debug_assert!(
            matches!(
                id,
                Err(ParseAccountError {
                    kind: ParseErrorKind::RedundantSeparator,
                    char: Some((12, '.'))
                })
            ),
            "{:?}",
            id
        );

        let id = AccountIdRef::new("jack__Quaid.near");
        debug_assert!(
            matches!(
                id,
                Err(ParseAccountError {
                    kind: ParseErrorKind::RedundantSeparator,
                    char: Some((5, '_'))
                })
            ),
            "{:?}",
            id
        );
    }

    #[test]
    fn test_is_valid_top_level_account_id() {
        let ok_top_level_account_ids = &[
            "aa",
            "a-a",
            "a-aa",
            "100",
            "0o",
            "com",
            "near",
            "bowen",
            "b-o_w_e-n",
            "0o0ooo00oo00o",
            "alex-skidanov",
            "b-o_w_e-n",
            "no_lols",
            "0123456789012345678901234567890123456789012345678901234567890123",
        ];
        for account_id in ok_top_level_account_ids {
            assert!(
                AccountIdRef::new(account_id).map_or(false, |account_id| account_id.is_top_level()),
                "Valid top level account id {:?} marked invalid",
                account_id
            );
        }

        let bad_top_level_account_ids = &[
            "ƒelicia.near", // fancy ƒ!
            "near.a",
            "b.owen",
            "bro.wen",
            "a.ha",
            "a.b-a.ra",
            "some-complex-address@gmail.com",
            "sub.buy_d1gitz@atata@b0-rg.c_0_m",
            "over.9000",
            "google.com",
            "illia.cheapaccounts.near",
            "10-4.8-2",
            "a",
            "A",
            "Abc",
            "-near",
            "near-",
            "-near-",
            "near.",
            ".near",
            "near@",
            "@near",
            "неар",
            "@@@@@",
            "0__0",
            "0_-_0",
            "0_-_0",
            "..",
            "a..near",
            "nEar",
            "_bowen",
            "hello world",
            "abcdefghijklmnopqrstuvwxyz.abcdefghijklmnopqrstuvwxyz.abcdefghijklmnopqrstuvwxyz",
            "01234567890123456789012345678901234567890123456789012345678901234",
            // Valid regex and length, but reserved
            "system",
        ];
        for account_id in bad_top_level_account_ids {
            assert!(
                !AccountIdRef::new(account_id)
                    .map_or(false, |account_id| account_id.is_top_level()),
                "Invalid top level account id {:?} marked valid",
                account_id
            );
        }
    }

    #[test]
    fn test_is_valid_sub_account_id() {
        let ok_pairs = &[
            ("test", "a.test"),
            ("test-me", "abc.test-me"),
            ("gmail.com", "abc.gmail.com"),
            ("gmail.com", "abc-lol.gmail.com"),
            ("gmail.com", "abc_lol.gmail.com"),
            ("gmail.com", "bro-abc_lol.gmail.com"),
            ("g0", "0g.g0"),
            ("1g", "1g.1g"),
            ("5-3", "4_2.5-3"),
        ];
        for (signer_id, sub_account_id) in ok_pairs {
            assert!(
                matches!(
                    (AccountIdRef::new(signer_id), AccountIdRef::new(sub_account_id)),
                    (Ok(signer_id), Ok(sub_account_id)) if sub_account_id.is_sub_account_of(signer_id)
                ),
                "Failed to create sub-account {:?} by account {:?}",
                sub_account_id,
                signer_id
            );
        }

        let bad_pairs = &[
            ("test", ".test"),
            ("test", "test"),
            ("test", "a1.a.test"),
            ("test", "est"),
            ("test", ""),
            ("test", "st"),
            ("test5", "ббб"),
            ("test", "a-test"),
            ("test", "etest"),
            ("test", "a.etest"),
            ("test", "retest"),
            ("test-me", "abc-.test-me"),
            ("test-me", "Abc.test-me"),
            ("test-me", "-abc.test-me"),
            ("test-me", "a--c.test-me"),
            ("test-me", "a_-c.test-me"),
            ("test-me", "a-_c.test-me"),
            ("test-me", "_abc.test-me"),
            ("test-me", "abc_.test-me"),
            ("test-me", "..test-me"),
            ("test-me", "a..test-me"),
            ("gmail.com", "a.abc@gmail.com"),
            ("gmail.com", ".abc@gmail.com"),
            ("gmail.com", ".abc@gmail@com"),
            ("gmail.com", "abc@gmail@com"),
            ("test", "a@test"),
            ("test_me", "abc@test_me"),
            ("gmail.com", "abc@gmail.com"),
            ("gmail@com", "abc.gmail@com"),
            ("gmail.com", "abc-lol@gmail.com"),
            ("gmail@com", "abc_lol.gmail@com"),
            ("gmail@com", "bro-abc_lol.gmail@com"),
            (
                "gmail.com",
                "123456789012345678901234567890123456789012345678901234567890@gmail.com",
            ),
            (
                "123456789012345678901234567890123456789012345678901234567890",
                "1234567890.123456789012345678901234567890123456789012345678901234567890",
            ),
            ("aa", "ъ@aa"),
            ("aa", "ъ.aa"),
        ];
        for (signer_id, sub_account_id) in bad_pairs {
            assert!(
                !matches!(
                    (AccountIdRef::new(signer_id), AccountIdRef::new(sub_account_id)),
                    (Ok(signer_id), Ok(sub_account_id)) if sub_account_id.is_sub_account_of(&signer_id)
                ),
                "Invalid sub-account {:?} created by account {:?}",
                sub_account_id,
                signer_id
            );
        }
    }

    #[test]
    fn test_is_account_id_64_len_hex() {
        let valid_64_len_hex_account_ids = &[
            "0000000000000000000000000000000000000000000000000000000000000000",
            "6174617461746174617461746174617461746174617461746174617461746174",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "20782e20662e64666420482123494b6b6c677573646b6c66676a646b6c736667",
        ];
        for valid_account_id in valid_64_len_hex_account_ids {
            assert!(
                matches!(
                    AccountIdRef::new(valid_account_id),
                    Ok(account_id) if account_id.is_implicit()
                ),
                "Account ID {} should be valid 64-len hex",
                valid_account_id
            );
        }

        let invalid_64_len_hex_account_ids = &[
            "000000000000000000000000000000000000000000000000000000000000000",
            "6.74617461746174617461746174617461746174617461746174617461746174",
            "012-456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "fffff_ffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "oooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo",
            "00000000000000000000000000000000000000000000000000000000000000",
        ];
        for invalid_account_id in invalid_64_len_hex_account_ids {
            assert!(
                !matches!(
                    AccountIdRef::new(invalid_account_id),
                    Ok(account_id) if account_id.is_implicit()
                ),
                "Account ID {} is not an implicit account",
                invalid_account_id
            );
        }
    }

    #[test]
    #[cfg(feature = "arbitrary")]
    fn test_arbitrary() {
        let corpus = [
            ("a|bcd", None),
            ("ab|cde", Some("ab")),
            ("a_-b", None),
            ("ab_-c", Some("ab")),
            ("a", None),
            ("miraclx.near", Some("miraclx.near")),
            (
                "01234567890123456789012345678901234567890123456789012345678901234",
                None,
            ),
        ];

        for (input, expected_output) in corpus {
            assert!(input.len() <= u8::MAX as usize);
            let data = [input.as_bytes(), &[input.len() as _]].concat();
            let mut u = arbitrary::Unstructured::new(&data);

            assert_eq!(
                u.arbitrary::<&AccountIdRef>()
                    .ok()
                    .map(AsRef::<str>::as_ref),
                expected_output
            );
        }
    }
}