use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{LazyLock, RwLock, RwLockReadGuard},
};

use tokio::{fs::File, io::AsyncReadExt};

// 辞書のパスを設定する (下のURLの bep-eng.dic を入れる)
// https://fastapi.metacpan.org/source/MASH/Lingua-JA-Yomi-0.01/lib/Lingua/JA
const BEPENG_DIC_URL: &str =
    "https://fastapi.metacpan.org/source/MASH/Lingua-JA-Yomi-0.01/lib/Lingua/JA/bep-eng.dic";
pub const BEPENG_DIC_PATH: &str = "./appdata/downloads/bep-eng.dic";
pub const BEPENG_DIC_FOLDER: &str = "./appdata/downloads";

static TRANS_DICT: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct EngToKana<'a> {
    dict_data: RwLockReadGuard<'a, HashMap<String, String>>,
}

impl EngToKana<'_> {
    pub async fn download_init_dic() -> anyhow::Result<()> {
        /* 辞書のダウンロード */

        // すでにダウンロードしてある場合ダウンロードしない
        let dic_path = PathBuf::from(BEPENG_DIC_PATH);
        if !dic_path.exists() {
            log::info!("Downloading kana reading dictionary...");

            // ダウンロード
            let response = reqwest::get(BEPENG_DIC_URL).await?;
            let bytes = response.bytes().await?;

            // フォルダ作成と書き込み
            tokio::fs::create_dir_all(BEPENG_DIC_FOLDER).await?;

            let mut file = File::create(BEPENG_DIC_PATH).await?;
            tokio::io::copy(&mut bytes.as_ref(), &mut file).await?;

            log::info!("Kana reading dictionary download is complete.");
        }

        /* static TRANS_DICT の初期化 */

        let mut bepeng_dic = File::open(BEPENG_DIC_PATH).await?;
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
            let mut lock = TRANS_DICT.write().unwrap();
            *lock = trans_dict
        }

        Ok(())
    }

    /// 入力された文章の英語をすべてカタカナ読みに変換する。
    ///
    /// もし "VeryVeryExcellent" のように英単語がつながっていても、
    ///
    /// 単語ごとに分割して、"ベリーベリーエクセレント" のように変換する。
    pub fn convert_all(target_text: &str) -> String {
        let lock = TRANS_DICT.read().unwrap();
        let engtokana = Self { dict_data: lock };

        let mut result_words = vec![];

        for i in engtokana.split_en_other(target_text) {
            for splited in engtokana.split_word(i) {
                result_words.push(engtokana.convert_single_word(splited));
            }
        }

        let s = result_words.join("");
        return s;
    }

    /// 英語とそれ以外の単語で分割する
    ///
    /// あいうabcえお -> [あいう, abc, えお]
    fn split_en_other<'a>(&self, target_text: &'a str) -> Vec<&'a str> {
        let mut result_text = vec![];

        // 日本語と英語でそれぞれ分割して、その最初と最後の番目を取得する
        let separate_index: Vec<[usize; 2]> = {
            let mut boundaries = Vec::new();
            let mut chars = target_text.char_indices();

            if let Some((start, c)) = chars.next() {
                let mut is_ascii_alphabetic = c.is_ascii_alphabetic();
                let mut range_start = start;

                for (i, c) in chars {
                    if (c.is_ascii_alphabetic()) != is_ascii_alphabetic {
                        boundaries.push([range_start, i]);
                        range_start = i;
                        is_ascii_alphabetic = c.is_ascii_alphabetic();
                    }
                }
                boundaries.push([range_start, target_text.len()]);
            }

            boundaries
        };

        // 取得した位置からstrを取得して result_text に入れる
        for i in separate_index {
            result_text.push(&target_text[i[0]..i[1]]);
        }

        return result_text;
    }

    /// 単語単位で英語をカタカナ読みに変換する
    ///
    /// 変換できないものはそのまま返す
    pub fn convert_single_word<'a>(&'a self, word: &'a str) -> &'a str {
        if !self.is_convert_target(word) {
            return word;
        }

        let to_kana_result = self.get_kana_from_dict(word);

        // 変換可能だったら変換したもの、できなかったら入力をそのまま返す
        if let Some(kana) = to_kana_result {
            return kana;
        } else {
            return word;
        }
    }

    /// TRANS_DICT から対応する値を取り出す
    fn get_kana_from_dict(&self, key: &str) -> Option<&str> {
        if let Some(kana) = self.dict_data.get(&key.to_uppercase()) {
            Some(kana.as_str())
        } else {
            None
        }
    }

    /// 渡された単語が英語かつ、大文字だけではないなら True を返す
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

    /// つながった英単語を分割して Vec<str> にして返す
    ///
    ///  分割の必要がない場合やできなかった場合は入力をそのまま Vec に入れて返す
    ///
    ///  英語ではない場合も、Vec に入れてそのまま返す
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

    use crate::EngToKana;

    #[tokio::test]
    async fn test_download_dic() -> anyhow::Result<()> {
        EngToKana::download_init_dic().await?;
        EngToKana::download_init_dic().await?;

        Ok(())
    }

    #[tokio::test]
    async fn convert_all() -> anyhow::Result<()> {
        EngToKana::download_init_dic().await?;

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
}
