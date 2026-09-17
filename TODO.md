# crate分け方

一つ一つの責務をできるだけ小さくしたい。 sharedとかだるいので、やるなら型定義だけ。

- proxy
- round
- builder

データベースを誰に持たせて、複数持つなら誰に何の情報を持たせるのか考えないといけない。

9/14: 上流のReferencesを返すようにするー

- NarRecordをやめて、NarInfoに統合する
  - それはなんか微妙らしい。
  - どちらにせよ、NarRecord, NarInfoあたりの差異によって型が微妙になってるので直す
