/*
 * Copyright (C) 2024-2025 taylor.fish <contact@taylor.fish>
 *
 * This file is part of ncc.
 *
 * ncc is free software: you can redistribute it and/or modify it under
 * the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * ncc is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
 * or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General
 * Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public
 * License along with ncc. If not, see <https://www.gnu.org/licenses/>.
 */

#![deny(unsafe_code)]

use sha2::{Digest, Sha256};
use std::fmt::{self, Display};
use std::io::{self, Write};
use std::process::{Command, Stdio};

macro_rules! define_tests {
    (
        toml = $toml_prefix:literal,
        syx = $syx_prefix:literal
        $(, $func:ident($path:literal, $hash:literal $(,)?))* $(,)?
    ) => {
        $(#[test]
        fn $func() {
            TestCase {
                name: $path,
                toml: include_bytes!(concat!($toml_prefix, $path, ".toml")),
                syx: include_bytes!(concat!($syx_prefix, $path, ".syx")),
                toml_hash: hex_literal::hex!($hash),
            }
            .assert();
        })*
    };
}

define_tests! {
    toml = "../examples/",
    syx = "syx-data/",
    flkey_mini_colors_1(
        "flkey-mini/colors/1",
        "55acd05fb6aea7979c3d7ae5bbc6ebd037d006774aefcbcb37ec4cd6a0c0bfb5",
    ),
    flkey_mini_colors_2(
        "flkey-mini/colors/2",
        "85e878fb051cbda172cdf4f70f85d062fe8df08f946190652dfcdbcdc1a4af4f",
    ),
    flkey_mini_colors_3(
        "flkey-mini/colors/3",
        "1e9596ee6870de058fede3b172e14f4e64f5c0e45fb205b85afce92cf71f9afe",
    ),
    flkey_mini_colors_4(
        "flkey-mini/colors/4",
        "88255c759783be326c07ccaf08b64ab1bc2fdaeb9645459271dc8a56f39b966b",
    ),
    flkey_mini_colors_5(
        "flkey-mini/colors/5",
        "02899b194fd2dea8102e78dc6af79f7b9743beecd5acd920fb056097dee38ffd",
    ),
    flkey_mini_colors_6(
        "flkey-mini/colors/6",
        "25951f1c66f580b127145eb767fae85496f2bd16afae9ecfb6e641fd135176f4",
    ),
    flkey_mini_colors_7(
        "flkey-mini/colors/7",
        "2759a61f1de676eac4a2d17988e095e8434a20c41c0858d2e15c07ac918f4dea",
    ),
    flkey_mini_colors_8(
        "flkey-mini/colors/8",
        "a9e6b3b56d36af97857713098fb2688f83960e1ebde890d122e835f1cf99a623",
    ),
    flkey_mini_default_pads(
        "flkey-mini/default-pads",
        "fe81ed669b4168b55d12ffa9e5fc7ef7032eedebaf84dfffb2f93ec165c8c813",
    ),
    flkey_mini_default_pedal(
        "flkey-mini/default-pedal",
        "feed9c056ff3dadec9e022d236a5ce7a507c96e963deb0cce1f637daef09b5b1",
    ),
    flkey_mini_default_pots(
        "flkey-mini/default-pots",
        "b012a6072bfe5a6aa0bd47f61a5300e2157dfd05faa7d855af1609c6de5160a0",
    ),
    flkey_mini_example_pads(
        "flkey-mini/example-pads",
        "e8015c1716ab08edf3cc86556f29207d16c7b65f47b28e763d82535d0434b4b2",
    ),
    flkey_mini_example_pedal(
        "flkey-mini/example-pedal",
        "12e9490eedf15d1e1ef507a4bb79b4552ea8c3165f651cf6c8622791131fafff",
    ),
    flkey_mini_example_pots(
        "flkey-mini/example-pots",
        "dea9982181fa9acd42ec2c737f10573a77b0f4c26662304584553a2eeabc181b",
    ),
    flkey_colors_1(
        "flkey/colors/1",
        "64b2973f63181d2e037c42abce4b60ccc2d7b2c427dcb89afe1a1e0ef328e87c",
    ),
    flkey_colors_2(
        "flkey/colors/2",
        "b38df62d1054ececbe02c52ae174321635865d117c4813fe75e4d30ab3f150b1",
    ),
    flkey_colors_3(
        "flkey/colors/3",
        "8aca91d576f1c14c6bfd0fb9199470bbddcafedc4f3638406103e3f9b8f00724",
    ),
    flkey_colors_4(
        "flkey/colors/4",
        "1ec7714d2816b0c486cc3e1bb64e444da7a02bcd99c9133db1ff01c131cfcab8",
    ),
    flkey_colors_5(
        "flkey/colors/5",
        "c153a5b3e209720505c2b4cdd8d0c15916f82db9990ec96f3d17f9286873e502",
    ),
    flkey_colors_6(
        "flkey/colors/6",
        "80b93932b9e18a9004b8055e5ac923c59da4f5bee91f5c536abb0811f4be53cc",
    ),
    flkey_colors_7(
        "flkey/colors/7",
        "51fb57e3b88835b736484ed67ad75e1ce69bf8038e28d407f05d7a0ce8d8d826",
    ),
    flkey_colors_8(
        "flkey/colors/8",
        "4b37a0c42257a13248ef84ae6b75f012139436a1c76c81357a526ceccca09685",
    ),
    flkey_default_faders(
        "flkey/default-faders",
        "e695af682dc67cfc6a4480064286fca0334682e5bf450ba3c488908ce174387b",
    ),
    flkey_default_pads(
        "flkey/default-pads",
        "ff9982320968198862bb0683d5dc6d3936fea5e7a464df73d452845dabff3807",
    ),
    flkey_default_pedal(
        "flkey/default-pedal",
        "78bff2e9c2981de76638d19a4a24f7ff1bc5040ce68fb18829877eddf4452b74",
    ),
    flkey_default_pots(
        "flkey/default-pots",
        "72696d7403ede8e8ee9250d345aeb1ad207adedf87e4e3ba3b658ffbe1abddc2",
    ),
    flkey_example_faders(
        "flkey/example-faders",
        "5cdf46fb7531d8dc47ce91d1f9786b92009a9c4e2c7fee71801e28f681a8c7ad",
    ),
    flkey_example_pads(
        "flkey/example-pads",
        "56aca1e0ebd244bbcdc44a15c3af1f6710a7a9963284100ee548359b25b6b1af",
    ),
    flkey_example_pedal(
        "flkey/example-pedal",
        "4117c4e61f5a096a1e8183bba4082c0ef460a9020377b3ff9c828a0c24d6fd91",
    ),
    flkey_example_pots(
        "flkey/example-pots",
        "fb890e9e543d2f8abf37213b03fafd6793e21cc3a5ee07ef00135d5a4e6f76bf",
    ),
    launchkey_mini_mk3_colors_1(
        "launchkey-mini-mk3/colors/1",
        "0f7705a87e79c7d04bb6d1a566e40d9bfb69c9e206f7e504db7cd4e42e05605e",
    ),
    launchkey_mini_mk3_colors_2(
        "launchkey-mini-mk3/colors/2",
        "af3183f64d46f3b33512cbea6ad4015c7eb8df7bd8c07fb8aea20bd3e131cd45",
    ),
    launchkey_mini_mk3_colors_3(
        "launchkey-mini-mk3/colors/3",
        "6d050286b4aadd830521326bdc760645e015457d83ea003b4a6a5a4c26404fec",
    ),
    launchkey_mini_mk3_colors_4(
        "launchkey-mini-mk3/colors/4",
        "e173a43331279b76407a5254c1cffa05ffbae37d199f2bc377677f6a2439e5b7",
    ),
    launchkey_mini_mk3_colors_5(
        "launchkey-mini-mk3/colors/5",
        "362eb050f3c522f88049ede924cbfa50d50d3f6d047dbeff9119a4414c86f66b",
    ),
    launchkey_mini_mk3_colors_6(
        "launchkey-mini-mk3/colors/6",
        "22d5194a3abf6498c8928d938cdb7f2d71792d859b51fb598edf7c196337b27a",
    ),
    launchkey_mini_mk3_colors_7(
        "launchkey-mini-mk3/colors/7",
        "c297a806eaaff567db9020c24f020717aff2bf82eb9afc92a4776b5604edebb5",
    ),
    launchkey_mini_mk3_colors_8(
        "launchkey-mini-mk3/colors/8",
        "1e18343760b7c330354481a990f8caa8825802929afc6466d5bc0b55f6b61d53",
    ),
    launchkey_mini_mk3_default_pads(
        "launchkey-mini-mk3/default-pads",
        "4696e31c262a5b6686a47cfaf1d524d6b2d59a3460fa1a7ceaf7083fafdebe0c",
    ),
    launchkey_mini_mk3_default_pedal(
        "launchkey-mini-mk3/default-pedal",
        "79492e3ae5e35e56dc59123be6271c82db5c3416cc761f9d1a4b43111a22a797",
    ),
    launchkey_mini_mk3_default_pots(
        "launchkey-mini-mk3/default-pots",
        "112bdb87d16b11886b497b4d0cdc7fc7318e891bb3b012934831c4415e2b5344",
    ),
    launchkey_mini_mk3_example_pads(
        "launchkey-mini-mk3/example-pads",
        "f96ad6c96e53a200c43e0f7d2919b5d59aac2dc19c93d3371cd19111453cf8ed",
    ),
    launchkey_mini_mk3_example_pedal(
        "launchkey-mini-mk3/example-pedal",
        "cb010a2ffdebb1fb480573fff4c31448366e3689dde793bde96db908fe1b8b81",
    ),
    launchkey_mini_mk3_example_pots(
        "launchkey-mini-mk3/example-pots",
        "a2c8d9d289ad0dd228bb3de76f73e2c0d3fc5816fa9b9a5082d4caa665759d7f",
    ),
    launchkey_mk3_colors_1(
        "launchkey-mk3/colors/1",
        "0d939a9e67b7bd141e2903d25efab5a202ce4f23d6573fb2e321184de9dedb6b",
    ),
    launchkey_mk3_colors_2(
        "launchkey-mk3/colors/2",
        "af8e8e0a6ec0ec783a81dfde6145cee9660eee3b00dc805c973f3b78d88536a2",
    ),
    launchkey_mk3_colors_3(
        "launchkey-mk3/colors/3",
        "b651af537477518bbbe63669cac13e6982b65777f06c13fbabd2678490460bb8",
    ),
    launchkey_mk3_colors_4(
        "launchkey-mk3/colors/4",
        "4bd5f0c3ddfee1496c4250ff40e9e680931e63f87709fea4b4214263f0ee0bf0",
    ),
    launchkey_mk3_colors_5(
        "launchkey-mk3/colors/5",
        "d48bede0e196a45c46cf572762d9a78ff7c0bc2cb79cdfc1b2373765e5021149",
    ),
    launchkey_mk3_colors_6(
        "launchkey-mk3/colors/6",
        "07058750a0111a2bc02737249487d9d211dfc0e2f9f798f3afee76f987cadfe2",
    ),
    launchkey_mk3_colors_7(
        "launchkey-mk3/colors/7",
        "0d4ae74042dce1d00314231047e01550c4ba12d641e426688a3c7e73b67791f7",
    ),
    launchkey_mk3_colors_8(
        "launchkey-mk3/colors/8",
        "99617c72b67e2a875cb4a7c5d8d0a33ed92f80c8f211aebf864fc2270a8d245c",
    ),
    launchkey_mk3_default_faders(
        "launchkey-mk3/default-faders",
        "7f8548724d502046b4f5e663033354e0348a4f15c439e81d8d798b7f2261ad2c",
    ),
    launchkey_mk3_default_pads(
        "launchkey-mk3/default-pads",
        "3884aa19a661e6ce95a5992b76f89462dbb2b99b37bdf24fe02af12bf335aaff",
    ),
    launchkey_mk3_default_pedal(
        "launchkey-mk3/default-pedal",
        "cd25a98081d35c24913dd272eb38b30a1842ea1ff27e58697bc5fb22fc24671a",
    ),
    launchkey_mk3_default_pots(
        "launchkey-mk3/default-pots",
        "c10d7bb9e8cf07ad25125d8fcec292bb6b51f84bc58f5d850cc485b6be64f458",
    ),
    launchkey_mk3_example_faders(
        "launchkey-mk3/example-faders",
        "d2c441440c970f6d617e139d0efa59ea1506ff41dd4c98cd3b8714d519936d5f",
    ),
    launchkey_mk3_example_pads(
        "launchkey-mk3/example-pads",
        "da0ce543f676a191fbe6c44b49ee83907db79734e3e7cdf017848599137fee1c",
    ),
    launchkey_mk3_example_pedal(
        "launchkey-mk3/example-pedal",
        "1ee3155c3699768d4627f46bc548977508045c34964e55fae1b8e313b4bc0d94",
    ),
    launchkey_mk3_example_pots(
        "launchkey-mk3/example-pots",
        "3bc76de29149ea11a01ca08bd41273befbacca74c10061903403a53447031361",
    ),
    launchpad_mini_mk3_blank(
        "launchpad-mini-mk3/blank",
        "32e2b8800c3952883f763006890d9ac51fd08a82d5ed5532df72e3b2b92a5875",
    ),
    launchpad_mini_mk3_colors_1(
        "launchpad-mini-mk3/colors/1",
        "405e1e20ed8ad6178e1e25b80d61627cb3fe80a08d5f6c46f30864c1a10c01b4",
    ),
    launchpad_mini_mk3_colors_2(
        "launchpad-mini-mk3/colors/2",
        "3ee49499b5efe52b540170b45ef3914b65db33f080c95460ef3b5638ad2e011a",
    ),
    launchpad_mini_mk3_example(
        "launchpad-mini-mk3/example",
        "de159ffe5ec213061208e2f817ea82b57341cca7138cfa1f0084f401d083d34e",
    ),
    launchpad_x_blank(
        "launchpad-x/blank",
        "bbb805eab77c2563c6e970be63eb2ebb6256f7d5760809770a117dda4adfc794",
    ),
    launchpad_x_colors_1(
        "launchpad-x/colors/1",
        "1385e3b154460a046ffe591e46013935f6f58693cbe44d5e704040e29054a268",
    ),
    launchpad_x_colors_2(
        "launchpad-x/colors/2",
        "7179e5e54ba8b74e90d0beac715d1b96284a37167009cf691516d2020e606a5e",
    ),
    launchpad_x_example(
        "launchpad-x/example",
        "21977e39a0db3a453b8abd92b734fc5a8fc2802d95c08e3dabca41a1ee121e4d",
    ),
}

struct TestCase {
    name: &'static str,
    toml: &'static [u8],
    syx: &'static [u8],
    toml_hash: [u8; 32],
}

impl TestCase {
    pub fn run(&self) -> Result<(), Fail> {
        let mut hasher = Sha256::new();
        hasher.update(self.toml);
        let hash: [u8; 32] = hasher.finalize().into();
        if hash != self.toml_hash {
            return Err(Fail::BadHash);
        }
        let mut child = Command::new("ncc")
            .args(["-", "-o-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(Fail::Spawn)?;
        let mut stdin = child.stdin.take().unwrap();
        let output = (|| {
            stdin.write_all(self.toml)?;
            stdin.flush()?;
            drop(stdin);
            child.wait_with_output()
        })()
        .map_err(Fail::ChildIo)?;
        if !output.status.success() {
            return Err(Fail::ChildStatus);
        }
        if output.stdout != self.syx {
            return Err(Fail::BadSyx);
        }
        Ok(())
    }

    pub fn assert(&self) {
        if let Err(e) = self.run() {
            panic!("error running test \"{}\": {e}", self.name);
        }
    }
}

#[derive(Debug)]
enum Fail {
    BadHash,
    Spawn(io::Error),
    ChildIo(io::Error),
    ChildStatus,
    BadSyx,
}

impl Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadHash => {
                write!(f, "hash of toml file does not match expected value")
            }
            Self::Spawn(e) => write!(f, "could not create child process: {e}"),
            Self::ChildIo(e) => {
                write!(f, "could not communicate with child process: {e}")
            }
            Self::ChildStatus => {
                write!(f, "child process exited unsuccessfully")
            }
            Self::BadSyx => {
                write!(f, "compiled sysex differs from expected value")
            }
        }
    }
}
