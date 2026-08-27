## [2026-08-27] Gitleaks icin Go calistiricisi sistemde yoktu

Sebep: Git gecmisi gizli bilgi taramasi icin `go run ...gitleaks` denendi ancak sistemde `go` komutu kurulu degildi.

Cozum: Gitleaks'in resmi GitHub surumunun macOS arm64 paketi `/private/tmp` altina indirildi, yayinlanan SHA-256 listesiyle dogrulandi ve depo/gecmis taramasi gecici binary ile tamamlandi.

Kural: Guvenlik tarayicisi sistemde yoksa kalici kurulum yapmadan once resmi imzali/checksum'li gecici binary yolunu kullan; gizli eslesme degerlerini loglama.

## [2026-08-26] Iptal edilen tarama noop indeksle devam etti

Sebep: Duraklatma aktif taramayi iptal etti ancak devam etme akisi bos `SearchCache::noop` uzerinde yalnizca FSEvents watcher baslatiyordu.

Cozum: Paylasilan `rebuild_required` durumu eklendi; iptal edilen ilk/config taramasi ve FSEvents gecmis kaybi devam etmede zorunlu tam rebuild tetikliyor.

Kural: Uzun bir indeks kurulumunu iptal eden kontrol, devam etme yolunda eksik snapshot'i otomatik ve tam olarak yeniden kurmali.

## [2026-08-26] Sort comparator calisirken iptal nedeniyle degisti

Sebep: Metadata sort comparator'i her karsilastirmada generation okuyup iptalden sonra `Equal` donuyordu; bu, sort algoritmasinin total-order sozlesmesini bozuyordu.

Cozum: Sabit comparator kullanan chunk sort ve kontrollu merge uygulandi; iptal yalnizca chunk/merge sinirlarinda kontrol ediliyor ve sonraki siralama regresyon testi eklendi.

Kural: Iptal kontrolu comparator sonucunu ayni sort calismasi icinde degistirmemeli; kontrol algoritmanin guvenli safha sinirlarinda yapilmali.

## [2026-08-26] Favori toplu ekleme ayni yolu iki kez kaydetti

Sebep: Toplu eklemede bilinen yollar kumesi yalnizca onceki state'ten olusturuldu; ayni batch icindeki ilk yeni yol kumeye eklenmedigi icin yinelenen ikinci yol da kabul edildi.

Cozum: Batch tek geciste islenirken kabul edilen her yolu ayni anda bilinen yollar kumesine ekledim ve regresyon testini yesile cevirdim.

Kural: React state batch islemlerinde deduplikasyon kumesini yalnizca onceki state'ten degil, ayni geciste kabul edilen kayitlarla da guncelle.

## [2026-08-26] Globstar workspace testi tek seferlik siralama farkiyla dustu

Sebep: `search-cache` icindeki `test_globstar_dedup_trailing_expansion` tam workspace kosusunda ebeveyn `a` girdisini bir kez fazladan dondurdu; ayni hedef test arka arkaya iki kez yalitilmis kosuda gecti. Degisiklikler `search-cache` koduna dokunmuyordu.

Cozum: Hedef testi `--exact` ile iki kez yeniden calistirip gecici/testler-arasi durum oldugunu ayirdim; Cardinal'in 46 Rust testi ayrica gecti.

Kural: Tam workspace kosusunda tek arama testi duserse once ayni testi `-p search-cache --lib <tam_ad> -- --exact` ile iki kez dogrula; yalitilmis kosu da duserse urun hatasi olarak ele al.

## [2026-08-22] Vitest filtresinde proje yolu tekrarlandi

Tekrar: 2 (2026-08-27, `cardinal/` calisma dizininde filtreye yeniden `cardinal/src/...` verildi.)

Sebep: Calisma dizini `cardinal/` iken test filtresine `cardinal/src/...` verildi.

Cozum: Bu dizinden `npm test -- --run src/hooks/__tests__/useRemoteSort.test.ts` kullan.

Kural: Test dosyasi filtresini her zaman test komutunun `workdir` degerine gore yaz.

## [2026-08-22] Cargo fmt manifest yolu iki kez yazildi

Tekrar: 3 (2026-08-27, `cardinal/` calisma dizininde manifest yolu `cardinal/src-tauri/...` olarak tekrar yazildi.)

Sebep: Calisma dizini zaten `cardinal/src-tauri` iken manifest yolu tekrar `cardinal/src-tauri/Cargo.toml` verildi.

Cozum: Bu dizinden `cargo fmt --manifest-path Cargo.toml` kullan.

Kural: Mutlak veya goreli manifest yolu vermeden once komutun `workdir` degerini hesaba kat.

## [2026-08-22] Cargo PATH icinde bulunamadi

Sebep: Masaustu Codex kabugunun PATH degeri kullanicinin Rust arac dizinini icermiyor.

Cozum: Bilinen kurulum yollarinda da Rust bulunmadigi icin resmi Rustup arac zincirini kur, sonra repo tarafindan sabitlenen toolchain'i yukle.

Kural: Rust testlerinden once `command -v cargo` ve bilinen kurulum yollarini dogrula; eksik arac hatasini test RED kaniti sayma.

## [2026-08-22] Graphify headless extract API anahtari istedi

Sebep: Kurulu `graphify extract` komutu kod agirlikli kapsamda bile semantic backend anahtari olmadan baslamiyor.

Cozum: Yalnizca kod mimarisi gereken durumda Graphify'nin deterministik AST `extract` API'sini dogrudan kullan ve semantic adimi atla.

Kural: API anahtari bulunmayan yerel kod depolarinda once AST-only hizli yolu kullan.

## [2026-08-22] Graphify yol argumani alt komut sanildi

Sebep: Kurulu Graphify CLI yol tabanli dogrudan calistirma yerine `extract <path>` alt komutunu bekliyor.

Cozum: Headless kod analizi icin `graphify extract <path> --out <dir>` kullan.

Kural: Kurulu Graphify surumunun `--help` ciktisini kontrol etmeden skill dokumanindaki eski dogrudan yol sozdizimini kullanma.

## [2026-08-22] Graphify sistem Python modülü bulunamadı

Sebep: `graphify` CLI ayrı bir `uv tool` ortamında kurulu olmasına rağmen ilk tespit komutu sistem `python3` yorumlayıcısıyla çalıştırıldı.

Çözüm: Graphify yorumlayıcısını `uv tool run graphifyy python` ile çözüp sonraki komutlarda `graphify-out/.graphify_python` üzerinden kullan.

Kural: Graphify Python API çağrılarında sistem `python3` varsayma; skill'in yorumlayıcı çözümleme adımını önce çalıştır.
## [2026-08-22] Node 26 deneysel localStorage jsdom ortamını ezdi
Sebep: Node v26'nın deneysel global Web Storage uygulaması, Vitest/jsdom içindeki `window.localStorage` yerine dosya yolu verilmemiş bir depolama nesnesi sundu.
Çözüm: Testleri `NODE_OPTIONS=--no-experimental-webstorage` ile çalıştırarak jsdom'un kendi localStorage uygulamasını kullandım.
Kural: Node 26 üzerinde jsdom tabanlı Cardinal testlerinde bu ortam seçeneğini kullan; uygulama hatası sanıp test kodunu değiştirme.
