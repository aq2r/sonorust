English (Used Google or DeepL translate) | [日本語](./README-ja.md)

# sonorust
A Discord bot that can read aloud using `server_fastapi.py`

[litagin02/Style-Bert-VITS2](https://github.com/litagin02/Style-Bert-VITS2) or sbv2_core from [tuna2134/sbv2-api](https://github.com/tuna2134/sbv2-api).

Download: [Releases](https://github.com/aq2r/sonorust/releases)

## App features

- Change `Model`, `Speaker` and `Style` for each user [^1]

- When using litagin02/Style-Bert-VITS2, the API will be automatically started when the app starts [^2]

- When using tuna2134/sbv2-api, the necessary models, ONNXRuntime, etc. are automatically downloaded [^2]

- Change prefix

- Functions such as automatic voice chat participation and server dictionary

- Supports Japanese and English [^3]

[^1]: Speaker and Style switching is only supported by litagin02/Style-Bert-VITS2
[^2]: Only supported by Windows, operation not confirmed on other platforms
[^3]: Google Translate and DeepL Translate are used for English.

## How to use and feature explanation

[Sonorust Wiki](https://github.com/aq2r/sonorust/wiki) (Japanese Only)

## Link

Style-Bert-VITS2: https://github.com/litagin02/Style-Bert-VITS2

sbv2-api: https://github.com/tuna2134/sbv2-api

(This text-to-speech BOT uses a modified version of the core part of sbv2-api: https://github.com/aq2r/sbv2_core )

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
