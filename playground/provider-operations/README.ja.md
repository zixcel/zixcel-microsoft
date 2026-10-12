# microsoftの隔離操作playground

`packages/provider-microsoft` で `npm ci --ignore-scripts`、`npm test`、`node ../../.github/scripts/provider-consumer.mjs` を実行します。owner CIとOIDC公開前のgateでも実行します。

実候補をpackし、空npm設定の別consumerへpublic registry依存と導入して、公開型・読み書き・拒否・対象束縛・credential反射拒否・public importの実行例を検証します。account・credential・HTTP状態は架空で、cloud CLI・host keyring・実accountに触れません。

`demo.mjs` は限定変更のplan/承認、別承認のfake read、credential再取得を伴わないreplay拒否を示します。fake transportは同じローカルobjectを返すため、実providerのread-back手順ではありません。実受入時は対象resourceの読取り操作と事実を独立検証します。

ownerアプリがaccount/resource/action、source/package版、grant期限/失効、extension catalog、read-back、不明な結果の扱いを持ちます。生成コードにはexecute portのみを渡し、approve/credentialを渡しません。必要権限はmetadataであり完全IAM観測ではありません。invoice正規化・継続監視・完全IAM discoveryは今後の機能で、fixture完了とは分けます。

[English](README.md)

公開workflowはmanifestから版とarchiveを取得します。再実行時は既存版のintegrityとprovenanceが候補と一致する場合だけ再利用し、新たなOIDC公開済みとは扱いません。不一致は拒否します。配布payload変更時は版を進め、既存releaseを変更しません。
