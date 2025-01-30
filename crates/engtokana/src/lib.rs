use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{LazyLock, RwLock, RwLockReadGuard},
};

use tokio::{
    fs::File,
    io::{AsyncReadExt as _, AsyncWriteExt as _},
};

// 辞書のパスを設定する (下のURLの bep-eng.dic を入れる)
// https://fastapi.metacpan.org/source/MASH/Lingua-JA-Yomi-0.01/lib/Lingua/JA
const BEPENG_DIC_URL: &str =
    "https://fastapi.metacpan.org/source/MASH/Lingua-JA-Yomi-0.01/lib/Lingua/JA/bep-eng.dic";

static TRANS_DICT_MAP: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct EngToKana<'a> {
    dict_data: RwLockReadGuard<'a, HashMap<String, String>>,
}

impl EngToKana<'_> {
    pub async fn download_and_init_dic<P>(download_to_path: P) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        let download_to_path: PathBuf = download_to_path.into();
        let bepeng_dic_path = download_to_path.join("bep-eng.dic");

        // ダウンロードされていない場合のみダウンロードする
        if !bepeng_dic_path.exists() {
            let response = reqwest::get(BEPENG_DIC_URL).await?;
            let bytes = response.bytes().await?;

            let mut buffer = File::create(&bepeng_dic_path).await?;
            buffer.write_all(&bytes).await?;
        }

        let mut bepeng_dic = File::open(bepeng_dic_path).await?;

        let mut dic_text = String::new();
        bepeng_dic.read_to_string(&mut dic_text).await?;

        // 改行で区切って、空白行、コメント行を消す
        let vec: Vec<&str> = dic_text
            .split("\n")
            .filter(|s| *s != "")
            .filter(|s| !s.starts_with("#"))
            .collect();

        let mut trans_dict: HashMap<String, String> = HashMap::with_capacity(vec.len());

        // それぞれの要素を空白で前半と後半で分割して、trans_dict に組み合わせを登録
        vec.iter().for_each(|s| {
            let split: Vec<&str> = s.split(" ").collect();
            let before = split[0].to_string();
            let after = split[1].to_string();

            trans_dict.insert(before, after);
        });

        // static 変数を更新
        {
            let mut lock = TRANS_DICT_MAP.write().unwrap();
            *lock = trans_dict
        }

        Ok(())
    }

    fn new() -> Self {
        let lock = TRANS_DICT_MAP.read().unwrap();
        Self { dict_data: lock }
    }

    pub fn convert_all(text: &str) -> String {
        let slf = Self::new();
        let mut result_words = vec![];

        for i in slf.split_en_other(text) {
            for splited in slf.split_word(i) {
                result_words.push(slf.convert_single_word(splited));
            }
        }

        let result_string = result_words.join("");
        return result_string;
    }

    fn split_en_other<'a>(&self, text: &'a str) -> Vec<&'a str> {
        let mut result_texts = vec![];

        let mut chars = text.char_indices().peekable();
        let mut range_start: usize = 0;

        while let Some((i, c)) = chars.next() {
            if let Some((next_i, next_c)) = chars.peek() {
                if c.is_ascii_alphabetic() != next_c.is_ascii_alphabetic() {
                    result_texts.push(&text[range_start..(*next_i)]);
                    range_start = i + c.len_utf8();
                }
            }
        }

        // 最後の範囲を追加
        if range_start < text.len() {
            result_texts.push(&text[range_start..]);
        }

        result_texts
    }

    fn convert_single_word<'a>(&'a self, word: &'a str) -> &'a str {
        if !self.is_convert_target(word) {
            return word;
        }

        let result = self
            .dict_data
            .get(&word.to_uppercase())
            .map(|i| i.as_str())
            .unwrap_or_else(|| word);
        result
    }

    // 渡された単語が英語かつ、大文字だけではないなら True を返す
    fn is_convert_target(&self, word: &str) -> bool {
        // 英語かどうか
        if word.is_ascii() {
            //大文字だけかどうか
            if word.chars().all(|c| c.is_uppercase()) {
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    fn split_word<'a>(&self, target_str: &'a str) -> Vec<&'a str> {
        if !self.is_convert_target(target_str) {
            return vec![&target_str]; // 変換したい文字ではない場合、そのまま返す
        }

        let target_str_upper = &target_str.to_uppercase();
        if self.dict_data.contains_key(target_str_upper) {
            return vec![&target_str]; // target_word が辞書にある場合そのまま返す
        }

        // target_word の1文字目から、文字数-1文字目, 文字数-2文字目, ... と見ていく
        for i in (0..target_str.chars().count()).rev() {
            let target_prefix = &target_str_upper[..i];

            // target_word の最初の方が辞書にあったら
            if self.dict_data.contains_key(target_prefix) {
                let target_suffix = &target_str[i..];

                // target_word から見つかった単語を引いて、まだ文字が残っているなら
                if !target_suffix.is_empty() {
                    let target_suffix_split = self.split_word(target_suffix); // さらに残りを再帰で分割する

                    let mut vec = vec![&target_str[..i]];
                    vec.extend(target_suffix_split);
                    return vec;
                } else {
                    return vec![&target_str[..i]];
                }
            }
        }

        return vec![&target_str]; // 分割できなかったら入力をそのまま返す
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use tokio::fs::create_dir_all;

    use super::*;

    #[tokio::test]
    async fn convert_all() -> anyhow::Result<()> {
        create_dir_all("./appdata").await?;
        EngToKana::download_and_init_dic("./appdata").await?;

        let now = Instant::now();
        let result = EngToKana::convert_all("Hello");
        dbg!(now.elapsed());
        assert_eq!("ハロー", result);

        let now = Instant::now();
        let result = EngToKana::convert_all("Hello");
        dbg!(now.elapsed());
        assert_eq!("ハロー", result);

        let now = Instant::now();
        let result = EngToKana::convert_all("こんにちはworld！");
        dbg!(now.elapsed());
        assert_eq!("こんにちはワールドゥ！".to_string(), result);

        let now = Instant::now();
        let result = EngToKana::convert_all("veryveryexcellent");
        dbg!(now.elapsed());
        assert_eq!("ベリーベリーエクセレントゥ", result);

        let now = Instant::now();
        let result = EngToKana::convert_all("veryveryexcellentveryveryexcellentveryveryexcellent");
        dbg!(now.elapsed());
        assert_eq!(
            "ベリーベリーエクセレントゥベリーベリーエクセレントゥベリーベリーエクセレントゥ",
            result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_split_en_other() -> anyhow::Result<()> {
        let engtokana = EngToKana::new();

        let result = engtokana.split_en_other("あいう;abcえお_def");
        assert_eq!(result, vec!["あいう;", "abc", "えお_", "def"]);

        Ok(())
    }

    #[tokio::test]
    async fn test_download_and_init_dic() -> anyhow::Result<()> {
        create_dir_all("./appdata").await?;
        EngToKana::download_and_init_dic("./appdata").await?;

        let path = PathBuf::from("./appdata/bep-eng.dic");
        assert!(path.exists());

        Ok(())
    }
}
