## [2026-11-27] Boyut motoru arka planda tekrarlayan yuksek CPU kullandi

Sebep: Canli process sample `folder-size-worker -> process_pending -> scan_tree` ve getattrlistbulk okumalarini gosterdi. Motor 1,5 saniyelik dongude sekme/odak kontrolu olmadan calisiyordu; olculen kisa sureli CPU %555,9'a ulasti.

Cozum: Boyut sekmesi secimi native tarafa aktarildi. Sekme degisince aninda, odak kaybindan 15 saniye sonra tam ve artimli taramalar iptal edilir. Bekleyen olaylar korunur/sinirli kuyruga birlestirilir; tarama sadece izinli durumda devam eder. Eski snapshot acilista otomatik tarama baslatmaz.

Kural: Boyut motorunun calisma izni frontend gorunumunden ve native odaktan birlikte belirlenmeli; devam eden dosya okumalarinda da iptal kontrolu bulunmali. Dosya arama indeksinin ayri arka plan isi bu kuralla kapatilmaz.

## [2026-11-20] EminSet kurulumunda release makro kitapliklari yuklenemedi

Sebep: macOS linker/dyld, release proc-macro dylib dosyalarini `mis-aligned LINKEDIT string pool` nedeniyle reddetti; ardindan serde/phf/zerofrom icin crate bulunamadi hatalari goruldu. Paket bazli onbellek temizligi yeterli olmadi.

Cozum: Release yeniden `CARGO_PROFILE_RELEASE_STRIP=none npm run tauri build` ile derlendi. Uygulamanin uretim kaynak kodu degistirilmedi.

Kural: Bu makinede ayni linker hatasi tekrar ederse derleme profilindeki strip islemini kapatarak derle; eksik crate mesajini tek basina bagimlilik eksikligi sayma.

Ek dogrulama: 60 native ve 16 folder-size testi gecti. Frontend 342 test assertion'i gecti fakat App context-menu mock'undan 4 yakalanmamis TypeError kaydedildi; frontend kosusu temiz basari degildir. TypeScript kontrolu ayrica gecti.

## [2026-11-20] Kopya analizi testleri paralel generation sayacinda cakisti

Sebep: Paralel Rust testinde `full_hash_never_reads_past_the_remaining_budget`, baska bir testin ortak DUPLICATE_SCAN_GENERATION sayacini degistirmesi nedeniyle BudgetExceeded yerine Cancelled gordu.

Cozum: Ayni hedef test yalitilmis kosuda ve tum 57 test `--test-threads=1` ile gecti. Bu turda uretim/test kodu degistirilmedi; kalici test izolasyonu duzeltmesi henuz yapilmadi.

Kural: Devir/yayin dogrulamasinda sirali Rust komutunu ve paralel kosu sinirlamasini birlikte raporla; tek test gecisini paralel testlerin tam basarisi gibi sunma.

## [2026-11-01] WebKit GPU sureci kapandiktan sonra arayuz siyah kaldi

Sebep: macOS unified log 01:32-01:41 arasinda tekrarlayan GPUProcess Unresponsive ve GPUProcessCrashedTooManyTimes nedeniyle WebContent PID 1552'nin kapatildigini gosterdi. Ana surec acik kaldi; mevcut uygulamada arayuz saglik kontrolu/kurtarma yoktu. Onceki CPU artisinin ilk tetikleyicisi kayitlardan belirlenemedi.

Cozum: Gorunur pencerede native taraftan 15 saniyelik DOM/IPC kontrolu; 60 saniye yanitsizlikta WebView reload; 10 dakikada en fazla 3 deneme. Gizli/kucultulmus pencerede kurtarma yapilmaz. Saglik olaylari sinirli boyutlu loga kaydedilir. Kaybolan arama IPC alicisi arka plan indeksini panikletmez.

Kural: WebContent sureci ana uygulamadan bagimsiz sonlanabilir; native saglik kontrolu, sinirli kurtarma ve kayit tutulmali. GPU kilitlenmesinin ilk nedeni ile kurtarma eksikligi birbirinden ayri raporlanmali.

## [2026-08-28] Tauri target dizininde tasima oncesi mutlak yol kaldi

Sebep: Repo `.graphify/repos/...` yolundan `dev/cardinal` yoluna tasindiktan sonra Tauri'nin uretilmis izin dosyalari eski mutlak yolu isaret etmeye devam etti.

Cozum: `cargo clean --manifest-path src-tauri/Cargo.toml` ile yalniz yeniden uretilebilir Rust derleme onbellegi temizlendi ve test yeni konumda bastan derlendi.

Kural: Tauri reposu tasindiktan sonra ilk Rust derlemesinden once eski `target` ciktilarini bayat kopya olarak kabul et ve tam manifest yoluyla `cargo clean` calistir.

## [2026-08-28] Darwin getattrlist C probunda bildirim eksikti

Sebep: Ilk APFS klon probu `<sys/attr.h>` ekledi ancak `getattrlist` bildiriminin bulundugu `<unistd.h>` basligini eklemedi.

Cozum: Proba `<unistd.h>` eklendi; gercek APFS klonu ile tam kopya farkli clone-id ve paylasim bayraklariyla dogrulandi.

Kural: Darwin `getattrlist` problarinda hem `<sys/attr.h>` hem `<unistd.h>` ekle; derleme hatasini ozelligin desteklenmedigi seklinde yorumlama.

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

Tekrar: 3 (2026-08-28, `cardinal/` calisma dizininde kaynak okuma yoluna yeniden `cardinal/src/...` verildi.)

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
## [2026-08-28] Birden fazla remote tek git fetch komutunda ref gibi yorumlandi
Sebep: `git fetch --prune upstream origin` komutunda `origin`, ikinci remote yerine `upstream` uzerindeki ref adi olarak yorumlandi.
Cozum: Her remote ayri `git fetch --prune <remote>` komutuyla guncellendi.
Kural: Birden fazla Git remote guncellenecekse remote basina ayri fetch calistir.
## [2026-09-07] Native baglam menusundeki Cop onayi sessizce calismadi

Sebep: Native Tauri baglam menusu aksiyonu, WebView `window.confirm` cagrisini menu henuz kapanmadan eszamanli calistiriyordu; macOS onay penceresini gostermedi ve silme komutuna hic ulasilmadi.

Cozum: Onay ve `move_to_trash` akisi bir sonraki event-loop turuna ertelendi; gecikmeyi zorunlu kilan regresyon testi eklendi. Paketlenmis yeni imza macOS Tam Disk Erisimi yeniden verilmeden dosya listesini acmadigi icin fix sonrasi gercek UI silme kaniti henuz tamamlanmadi.

Kural: Native menu callback'lerinden WebView modal API'lerini ayni call stack icinde acma; menu kapandiktan sonraki event-loop turuna ertele ve gercek paketlenmis uygulamada dogrula.
