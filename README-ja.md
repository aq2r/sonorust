[English (Google or DeepL translate)](./README.md) | 日本語

# sonorust
Discord bot for Style-Bert-VITS2

SBV2の `server_fastapi.py` 用の Discord Botです。

ダウンロード: [releases](https://github.com/aq2r/sonorust/releases)

## 機能

- ユーザーごとの `Model`, `Speaker`, `Style` の変更

- プレフィックスの変更

- アプリ起動時に SBV2 の API を自動起動

- サーバー辞書といくつかのサーバーオプション

- 日本語と英語 (Google or DeepL translate) に対応

## 使用方法

ファイルを起動した後、表示に従って初期設定をします。

- デフォルト設定にするか
    
    - デフォルト設定でない場合、prefixやSBV2のURLなどを入力します。

- Botの言語設定

- SBV2の推論に使う言語設定

- 自動起動のためにSBV2のパスを設定するかどうか

- Bot Tokenの入力 (Developer Portal から Intent すべてをONにしておく必要があります)

## 基本的なコマンド

prefix - コマンド名 でBotのコマンドを使用できます。また、スラッシュコマンドからも使用できます。 (デフォルトでは `sn!` )

### help

Botのコマンド一覧を表示

### join

使用者がいるボイスチャンネルに参加

### leave

ボイスチャンネルから退席

<sub>
その他 10 個のコマンドは `help` コマンドから確認できます。
</sub>

<br>

## sbv2-api

https://github.com/tuna2134/sbv2-api を使用したい場合は、

初期設定画面で `tuna2134/sbv2-api` を選択し、`sbv2api_models` フォルダに sbv2 ファイルを入れてください。

## Link

Style-Bert-VITS2: https://github.com/litagin02/Style-Bert-VITS2

sbv2-api: https://github.com/tuna2134/sbv2-api

(この読み上げBOTでは sbv2-api の core 部分を改変して使用しています: https://github.com/aq2r/sbv2_core )

#

#### Lisense

<sub>

    Copyright (C) 2024 aq2r

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.

</sub>
