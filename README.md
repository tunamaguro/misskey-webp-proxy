# misskey-webp-proxy

RustによるMisskey Media Proxyの実装

## 使い方

[Releaseページ](https://github.com/tunamaguro/misskey-webp-proxy/releases)からバイナリを取得してください。その後バイナリを実行してください
```bash
$ misskey-webp-proxy --port 3000 --host 0.0.0.0 --quality-factor 75
```

## Docker

Dockerイメージは以下のように利用できます(https://hub.docker.com/r/tunamaguro/misskey-webp-proxy)

```bash
$ docker run tunamaguro/misskey-webp-proxy
```

## ライセンス

このプロジェクトはMITライセンスで提供されます