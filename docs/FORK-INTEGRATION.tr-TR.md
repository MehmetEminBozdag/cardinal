# Cardinal fork farkları ve entegrasyon rehberi

Güncelleme tarihi: **27 Kasım 2026**. Upstream `master` ve PR #228 yeniden kontrol edildi: taban hâlâ `4c50734f9a09d88110f96652b43634b412f79449`, PR açık ve birleştirilmemiş. Aşağıdaki 20 Kasım kayıtları tarihseldir; güncel eklemeler bu bölümde açıklanır.

## 27 Kasım eklemeleri ve yayın

Güncel yayın [v0.1.24-fork.2](https://github.com/MehmetEminBozdag/cardinal/releases/tag/v0.1.24-fork.2), kaynak dalı `public/cardinal-enhancements`. Paket Apple Silicon içindir; uygulama içindeki sürüm numarası `0.1.24` olarak kalır. DMG daha önce derlenip kurulan pakettir; sonraki commitler test ve belge içerir, üretim kaynaklarını değiştirmez.

EminSet'in `feature/app-uninstall` dalındaki [`a4658a1`](https://github.com/EminSet/cardinal/commit/a4658a17ffabd708dda08508aee93789f3aeaa30) commitine kadarki çalışma alınmıştır; bu dal klasör boyutu eklemelerini de içerir. 27 Kasım kontrolünde ilgili dalın ucu aynı committir.

- `folder-size/`: klasör boyutu motoru, tam/artımlı tarama, snapshot; UI'da liste, treemap, boş disk/Çöp boyutu ve temizlik önerileri. Mantıksal boyut, ayrılmış alan ve kesin geri kazanılacak alan aynı kavram değildir.
- Uygulama kaldırma ve olası sahipsiz kalıntılar: önerilen yollar kullanıcı tarafından incelenir; otomatik güvenli silme garantisi değildir. Editör/Chrome/pnpm/Codex önbelleği ve eski uzantı sürümü önerileri de gelir.
- `file-operations.jsonl`: Çöp'e taşıma işlemleri oturumlar arasında tutulur, yaklaşık 2 MiB sınırında döndürülür. Eski rehberin oturum içi işlem geçmişi açıklamasının yerini alır; geri alma sistemi değildir.
- `size_activity.rs`, `folder_sizes.rs`, `useFolderSizes.ts`: Boyut sekmesi + native pencere odağı birlikte tarama iznini belirler. Sekmeden çıkınca hemen; uygulama arka plana alındıktan **15 saniye** sonra devam eden tam/artımlı tarama iptal edilir. Tekrarlanan arka plan bildirimleri süreyi yenilemez. Bekleyen olaylar birleştirilir; 5.000 yol sınırı aşılırsa tek kök yenilemeye dönüştürülür. Uygun görünüm geri gelince yarım kalan tam tarama yeniden başlar. Arama indeksi ayrı çalışır.

Bu geliştirme sırasında 66 native, 27 folder-size ve iki hook testi, TypeScript, Clippy, biçim kontrolleri ve paket derlemesi geçti. Genel frontend koşusunda dört yakalanmamış context-menu mock hatası kaydedildi; tüm frontend testleri temiz geçti denemez. Son süre-sınırı testi kurulumdan sonra eklendi; üretim kodunu değiştirmedi. Kontrollü tam indeks CPU karşılaştırması yapılmadı. Kullanıcı mevcut kurulu uygulamanın düzgün çalıştığını bildirdi; bu bildirim bağımsız bir uçtan uca test değildir.

Yerel kaynak checkout'u `~/dev/cardinal`; kurulum/durum yedekleri Git'e dahil edilmeyen `.local-backups/` altında tutulur. Dağıtım DMG'si GitHub Release varlığıdır, Git kaynak ağacına eklenmez.

## Kullanılacak dal

Depo: <https://github.com/MehmetEminBozdag/cardinal>.

**Varsayılan ve tam entegrasyon dalı `public/cardinal-enhancements`.** Başlı başına PR #228'i almak tüm geliştirmeleri getirmez. Yayımlanan sürüm [Latest release](https://github.com/MehmetEminBozdag/cardinal/releases/latest) bağlantısındadır. `master`, `events_include_patsh` ve `sort_preserve` dalları değişiklikleri zaten güncel dalda bulunduğu için temizlenir; uçları `archive/2026-11-27/` etiketleriyle korunur. Açık PR'ların kullandığı üç `contrib/*` dalı ve birleştirilmemiş kod taşıyan diğer dallar korunur.

```bash
git clone --branch public/cardinal-enhancements https://github.com/MehmetEminBozdag/cardinal.git
cd cardinal
git rev-parse HEAD
git remote add upstream https://github.com/cardisoft/cardinal.git
```

| Dal | İçerik ve kullanım |
| --- | --- |
| `public/cardinal-enhancements` | Tüm özellikler, EminSet eklemeleri ve boyut taraması CPU düzeltmesi; entegrasyon başlangıcı. |
| `contrib/1-search-performance-file-workflows` | Upstream PR #228: sıralama/iş akışı ve Çöp onayı düzeltmesi. |
| `contrib/2-workspace-menus-coverage` | İnceleme için hazırlanmış çalışma alanı/menü/kapsam takip dalı. |
| `contrib/3-apfs-duplicates-locales` | APFS kopya analizi ve dil eşitliği takip dalı. |

Takip dalları eski yığının üzerine aynı Çöp düzeltmesinin ayrı cherry-pick commitlerini taşır. Bunları sırayla merge/cherry-pick etmek yerine tam dalı temel alın; aynı değişiklikleri iki kere uygulamayın. Kasım WebKit kurtarması `contrib/*` dallarında bulunmaz. Bu rehber hazırlanırken tam dal upstream tabanının **11 commit ilerisindeydi, gerisinde commit yoktu**; belge/yayın commitleri bu sayıyı artırır.

## Orijinalden gelen ve bizim eklediğimiz özellikler

Rust indeks motoru, FSEvents tabanlı takip, Everything benzeri sorgu dili, içerik/etiket araması, Quick Look, Finder'da gösterme, dosya kopyalama, klasör kapsamı, tema ve çok dillilik temel projeden gelir. Bunları sıfırdan bu fork geliştirmiş gibi sunmayın. Yeni farklar aşağıdadır.

### 1. Görsel arama ve en yeniler

- `SearchFilters.tsx` tür, değiştirilme tarihi ve boyut için açılır kontroller ekler. Klasör, resim, video, ses, belge, PDF, arşiv, kod ve uygulama türleri vardır.
- Tarih seçenekleri bugün/son hafta/son ay/son yıl; boyut eşikleri 10 MB/100 MB/1 GB üzeridir.
- Kontroller sorgu dilindeki `type:`, `dm:` ve `size:` ifadelerini üretir. Kullanıcı bunları elle yazmak zorunda değildir; sorgu dilinin kendisi kaldırılmamıştır.
- Ana metin, görsel filtre sorgusu ve klasör kapsamı ayrı tutulup aramaya birlikte aktarılır. Kayıtlı arama geri yüklenirken bunlar da korunur.
- En Yeniler kontrolü değiştirilme tarihine göre sıralamayı açıp kapatır. Boş sorguda da tüm indeks sonuçlarına uygulanabilir.
- Ayrı serbest uzantı veya içerik filtresi kontrolü bu bileşende **yoktur**. Eski sürüm notlarındaki uzantı arayüzü ifadesi fazla geniştir; uzantı/içerik araması mevcut sorgu diliyle yapılabilir.

Kaynaklar: `cardinal/src/components/SearchFilters.tsx`, `SearchBar.tsx`, `cardinal/src/hooks/useFileSearch.ts`, `useRemoteSort.ts`, `cardinal/src/App.tsx`.

### 2. Büyük sonuç sıralaması ve arka plan çalışması

- Boyut/değiştirilme/oluşturulma sıralaması için kompakt metadata snapshot'ı kullanılır; yalnız sıralamak için her dosyanın yol/ikon gibi tüm ayrıntılarını açmak gerekmez.
- Metadata sıralaması parçalara ayrılır; iptal kontrolü karşılaştırıcı içinde sonucu değiştirmek yerine güvenli parça/birleştirme sınırlarında yapılır.
- Son metadata sıralama isteği tutulur, bildirim kanalı sınırlıdır; eski sıralama isteklerinin sınırsız büyük sonuç vektörleri biriktirmesi önlenir. Bu sınır tüm IPC kanallarının sınırlı olduğu anlamına gelmez.
- Varsayılan sıralama eşiği 20.000'dir, tercihlerden değiştirilebilir. Bu sayı tüm indeks için dosya sayısı limiti değildir.
- İndekslemeyi duraklat/devam et, iptal edilmiş ilk taramadan sonra gerektiğinde yeniden kurulum ve pencere/arama etkinliğine göre arka plan flush denetimi bulunur.
- Cmd+Q kapanışı cache flush'ını beklemeye bağlanmaz. En son değişikliklerin diske dayanıklılığı ayrı bir konudur; her çıkışta son dosya olayının mutlaka kalıcılaştırıldığı iddia edilmez.

**Çekirdek dosya adı eşleştirmesinde ölçülmüş bir hızlanma yoktur.** Everything ile aynı motor/latans, her durumda düşük CPU veya tüm dosyalarda anlık metadata sıralaması vaat edilmez.

Kaynaklar: `cardinal/src-tauri/src/background.rs`, `commands.rs`, `sort.rs`, `lib.rs`, `search-cache/src/cache.rs`, `cardinal/src/hooks/useRemoteSort.ts`.

### 3. Çalışma alanı, favoriler ve menüler

- Kayıtlı arama; ad, metin, filtre, klasör sorgusu, kapsam görünürlüğü ve harf duyarlılığını saklar. Aynı adla kayıt güncellenir.
- Favoriler seçili dosya yollarını topluca saklar, yinelenen yolları ayıklar; favori listesi bir dosya yedeği değildir.
- İşlemler paneli Çöp'e taşıma için çalışıyor/tamamlandı/başarısız durumlarını gösterir. 27 Kasım sürümünde kalıcı, döndürülen günlük eklenmiştir; geri alma sistemi değildir.
- Arama/Görünüm/İndeks menüleri panel geçişleri, en yeniler, dosya/olay sekmesi, yeniden tarama ve ayarlara erişim ekler. Çalışma alanı araç çubuğu gizlenebilir ve tercih saklanır.

Kaynaklar: `WorkspacePanel.tsx`, `useWorkspaceCollections.ts`, `useFileOperationLog.ts`, `useWorkspaceToolbarVisibility.ts`, `menu.ts`, `appMenuActions.ts`.

### 4. “Neden Bulunamadı?” ve hariç tutulanları dahil etme

Panel Tam Disk Erişimi, indeks durumu/sayısı, izleme kökü, hariç/dahil yollar ve etkin klasör kapsamını gösterir. Ayar ekranına geçilebilir.

“Aramaya dahil et” yalnız o sorgu için filtreyi gevşetmez: yolu `ignorePaths` listesinden çıkarır ve `includePaths` listesine ekleyerek izleme yapılandırmasını değiştirir. Yeni kapsamın taranması gerekebilir; disk erişim iznini atlamaz. Entegrasyonda bu düğmeyi kalıcı kapsam değişikliği olarak ele alın.

Kaynaklar: `App.tsx::handleIncludePath`, `WorkspacePanel.tsx`, `useIgnorePaths.ts`, `useIncludePaths.ts`, `commands.rs::set_watch_config`.

### 5. Tekli ve toplu Çöp'e taşıma

Seçili satırlar bağlam menüsüne hedef listesi olarak aktarılır; onaydan sonra `move_to_trash` çağrılır. Kalıcı silme veya otomatik kopya temizleme yapılmaz.

Rust tarafı 1–1.000 yol, mutlak yol, kök/ev dizini koruması ve `.`/`..` gibi alias bileşenleri için doğrulama uygular. Yinelenen ve üst klasörün zaten kapsadığı alt hedefleri ayıklar; taşıma `trash` crate'iyle yapılır. Bu doğrulama bir sandbox veya yarış durumlarına karşı tam güvenlik garantisi değildir; host uygulamanın dosya yetkileriyle çalışır.

macOS native menüsü kapanmadan senkron `window.confirm` çağrılması komutu sessizce boşa düşürüyordu. `902ed3d` düzeltmesi onayı sonraki event-loop turuna erteler. Hook testleri gerçek Finder/Çöp E2E testi yerine geçmez; 7 Eylül kaydında yeni imza için Tam Disk Erişimi sonrası son toplu silme tekrarının tamamlanmadığı belirtilmiştir. Yayın öncesi geçici iki dosyayla kaynakların yokluğu ve Çöp'teki varlığı ayrıca kontrol edilmelidir.

Kaynaklar: `useSelection.ts`, `useContextMenu.ts`, `commands.rs::move_to_trash` ve `validate_trash_paths`.

### 6. APFS farkındalıklı kopya analizi

- İsteğe bağlı olarak **mevcut arama sonuçlarında** çalışır; tüm diskte sürekli tarama servisi değildir.
- Önce boyut, sonra kısmi SHA-256, ardından tam dosya SHA-256 ile aynı içerik gruplarını doğrular. Kısmi hash tek başına kopya kararı değildir.
- Aynı içerik ve depolama paylaşımı farklı sorulardır. Dosyalar `apfsClone`, `hardLink`, `physicalCopy` veya `unknown` olarak etiketlenir.
- APFS klon kimliği/paylaşım bayrakları ve dosya kimliği okunabildiği ölçüde sınıflandırma yapılır. Veri yoksa veya paylaşım yorumu kesin değilse `unknown` kalır; bunu fiziksel kopya saymayın.
- Limitler: en fazla 20.000 sonuç adayı, 50 GiB mantıksal girdi ve 8 GiB toplam okuma bütçesi. Boş dosyalar analize alınmaz; yalnız bulutta duran dataless dosyalar okunmadan atlanır. Sonuçlarda `limited`, `cancelled`, taranan/atlanan sayıları taşınır.
- Sorgu sonucu değiştiğinde eski analiz iptal edilir. Silme için otomatik seçim yoktur; gösterilen dosya boyutunu kesin geri kazanılacak fiziksel alan olarak toplamayın.

Kaynaklar: `duplicate_files.rs`, `commands.rs::find_duplicates`, `useDuplicateFiles.ts`, `WorkspacePanel.tsx`.

### 7. Siyah ekran sonrasında kurtarma

1 Kasım makine loglarında GPU sürecinin tekrar tekrar yanıtsız kalması üzerine WebKit arayüz sürecini kapattı; ana uygulama açık kaldı. `c062eec` bunu takip eden kurtarmayı ekler:

- Native taraftan 15 saniyede bir, yalnız görünür/küçültülmemiş ana pencereye DOM/IPC probu.
- 60 saniye yanıtsızlıkta WebView reload; 10 dakikada en fazla üç deneme.
- `renderer_heartbeat` komutu; dönen DOM/IPC yanıtıyla sağlıklılık ölçümü.
- Uygulamanın log dizininde `renderer-health.log`; yaklaşık 1 MiB sınırını geçince bir önceki dosyaya döndürme.
- Arayüz kaybolunca sahipsiz kalan arama cevabı arka plan döngüsünü panikletmez.

1 Kasım'daki paketli testte bu uygulamaya ait WebContent süreci kontrollü kapatıldı; ardından reload ve `Renderer DOM and IPC responsive` kaydı görüldü. Bu, GPU sürücüsünü düzeltmez; CPU sıçramasının ilk sebebi kanıtlanmış değildir. Uygulamayı başka bir arayüze gömerken `main` pencere etiketi, Tauri global invoke nesnesi ve React `#root` kontrolü uyarlanmalıdır. Bir dakikalık uzun işlemin yanlışlıkla kurtarmayı tetikleme olasılığı entegrasyonda değerlendirilmelidir.

Kaynaklar: `renderer_watchdog.rs`, `renderer_health.rs`, `lib.rs`, `background.rs`.

### 8. Yerelleştirme

Eklenen UI anahtarları `cardinal/src/i18n/resources/` içindeki 15 dilde bulunur: ar-SA, de-DE, en-US, es-ES, fr-FR, hi-IN, it-IT, ja-JP, ko-KR, pt-BR, ru-RU, tr-TR, uk-UA, zh-CN, zh-TW. Anahtar eşitliği ve sürüm notu testleri vardır; bu, her çevirinin bir ana dil konuşuru tarafından denetlendiği anlamına gelmez.

0.1.24 çok dilli tarihsel sürüm notları: [release notes](release-notes/0.1.24.md). Eylül Çöp onayı ve Kasım WebKit düzeltmeleri ayrıca [CHANGELOG](../CHANGELOG.md) içinde açıklanır.

## Başka bir uygulamayla birleştirirken

| Katman | Taşınacak/uyarlanacak parçalar |
| --- | --- |
| UI | `App.tsx` koordinasyonu, filtre/panel bileşenleri, hook'lar, `App.css`, dil anahtarları birlikte alınmalı. |
| IPC | Komutlar `lib.rs` içinde register edilir; payload tipleri `cardinal/src/types/` ve Rust yapılarına göre korunmalı. `SlabIndex` indeks içi kimliktir, kalıcı dosya kimliği değildir. |
| Arka plan | Sıralama kuyruğu, generation/iptal, watcher, cache ve kapanış birlikte incelenmeli. Yalnız UI kopyalamak native davranışı getirmez. |
| macOS | FSEvents, Finder, Quick Look, AppKit, APFS `getattrlist`, izinler ve global kısayollar platforma bağımlıdır; başka işletim sistemine hazır taşınabilir paket değildir. |
| Ürün kimliği | Ayrı ürün için `tauri.conf.json` identifier/productName, Rust/package metaverisi, simgeler, menü/yardım URL'leri ve imzalama ayarları birlikte değişmeli. |
| Depolama | `com.cardinal.one` kimliği altında `cardinal.db`, pencere durumu, WebKit/localStorage ve loglar mevcut Cardinal ile çakışabilir. Geçişi kopya/backup üzerinde test edin. |

İndeks dosyası SQLite değildir; mevcut kaynakta Zstandard sıkıştırılmış snapshot biçimidir. Host uygulamanın SQLite veritabanı gibi açmayın ve uygulamalar arasında eşzamanlı paylaştırmayın. İndeksi yeniden oluşturulabilir, kayıtlı arama/favorileri korunması gereken kullanıcı tercihi olarak ele alın.

Önemli localStorage anahtarları: `cardinal.workspace.savedSearches`, `cardinal.workspace.favorites`, `cardinal.workspace.toolbarVisible`, `cardinal.watchRoot`, `cardinal.ignorePaths`, `cardinal.includePaths`, `cardinal.sortThreshold`, `cardinal.search.directoryScopeOpen`. Bunların taşınması uygulama kimliğini değiştirmekten bağımsız bir veri geçişidir.

Mevcut ürün adı `Cardinal`, bundle identifier `com.cardinal.one`, native sürüm `0.1.24`; frontend `package.json` sürümü `0.1.0` olarak kalmıştır. Paket sürümünü frontend package sürümünden çıkarmayın. `menu.ts` güncelleme bağlantısı hâlâ upstream releases sayfasını açar; fork paketini oradan güncellediğinizi varsaymayın.

MIT lisansı [LICENSE](../LICENSE) dosyasında korunur. Orijinal telif/lisans bildirimlerini ürünle taşıyın; bağımlılık lisansları ayrıca gözden geçirilmelidir. Fork'un resmi upstream tarafından onaylandığını ima etmeyin.

## Upstream kontrolü — 2026-11-20

- [Upstream master commit](https://github.com/cardisoft/cardinal/commit/4c50734f9a09d88110f96652b43634b412f79449): son commit 26 Haziran 2026 UTC, arama girişine erişilebilirlik etiketi eklemesi. Fork zaten bu commit tabanında; sonradan master'a birleşmiş yeni commit görülmedi.
- [Son resmi release v0.1.23](https://github.com/cardisoft/cardinal/releases/tag/v0.1.23): 24 Mart 2026 UTC. Master ile release aynı şey değildir; klasör kapsamı gibi sonraki master değişiklikleri bu fork'un özgün katkısı değildir.
- [Bizim PR #228](https://github.com/cardisoft/cardinal/pull/228): OPEN, mergedAt boş; son görünen güncelleme 8 Eylül. `ldm0` tarafından gelen görünür yanıt hâlâ arama performansı iddiasına yönelik 28 Ağustos yorumudur. Yeni maintainer cevabı veya resmi kabul kaydı görülmedi. PR başlığı iş akışı/büyük sonuç sıralaması/kullanılabilirlik olarak düzeltilmiştir.

Aşağıdaki **açık topluluk PR'ları birleştirilmemiştir**. Başlık/kapsamları entegrasyon için takip konusu olarak incelendi; kodları bu fork'a alınmadı ve doğrulanmış çözüm olarak sunulmuyor:

| PR | Yazar | İlgili konu |
| --- | --- | --- |
| [#226](https://github.com/cardisoft/cardinal/pull/226) | franciscoxc | Görsel filtreler, içerik snippet'leri, modern Node testleri; bizim filtre UI ile örtüşebilir. |
| [#233](https://github.com/cardisoft/cardinal/pull/233) | huangzhong0527 | Bağlam menüsünden silme; bizim Çöp akışıyla örtüşebilir. |
| [#234](https://github.com/cardisoft/cardinal/pull/234) | vlasky | Sorgu dilinin uç durumları. |
| [#235](https://github.com/cardisoft/cardinal/pull/235) | vlasky | Harf duyarlı vurgu ve başarısız satır ayrıntılarını yeniden alma. |
| [#236](https://github.com/cardisoft/cardinal/pull/236) | vlasky | Snapshot yazma sınırları ve eksik indeksin diske yazılmaması; persistence entegrasyonunda öncelikle incelenmeli. |
| [#237](https://github.com/cardisoft/cardinal/pull/237) | vlasky | Filtre sırası, eski watch config, sorted view, Quick Look borrow; aynı dosyalarda çatışma ihtimali var. |
| [#238](https://github.com/cardisoft/cardinal/pull/238) | a1003584293 | Yer imleri, içe/dışa aktarım, filtre ve görünüm değişiklikleri; çalışma alanıyla örtüşebilir. |

Bu PR'ların yazarlarını orijinal maintainer olarak sınıflandırmıyoruz; açık katkı olmaları resmi kabul anlamına gelmez.

## Dogrulama

Depo kökünden:

```bash
cd cardinal
npm ci
NODE_OPTIONS=--no-experimental-webstorage npm test
npm run typecheck
npm run build
cd ..
cargo test --manifest-path cardinal/src-tauri/Cargo.toml --lib -- --test-threads=1
cargo clippy --manifest-path cardinal/src-tauri/Cargo.toml --lib -- -D warnings
cargo fmt --manifest-path cardinal/src-tauri/Cargo.toml --check
```

`rust-toolchain.toml` mevcut kayıtta `nightly-2025-12-11` pinler. Kök AGENTS.md içindeki eski nightly örneği yerine bu dosyayı esas alın. `search-cache` gibi paylaşılan crate'leri değiştirirseniz ilgili crate testlerini ve workspace kontrollerini de çalıştırın.

Paket üretimi için depo kökünden `cd cardinal && npm run tauri build`. Bu makinede release proc-macro yüklemesi `mis-aligned LINKEDIT string pool` hatası verirse doğrulanmış komut `CARGO_PROFILE_RELEASE_STRIP=none npm run tauri build` şeklindedir. Çıktılar `cardinal/src-tauri/target/release/bundle/macos/` ve `bundle/dmg/` altındadır. Kaynak Git dalı DMG'yi içermez; dağıtım paketi GitHub Release varlığı olarak yayımlanır. Yerel ad-hoc imza Apple notarization değildir; yeni paket kimliği/imzası Tam Disk Erişimi'nin yeniden verilmesini gerektirebilir.

20 Kasım devir kontrolünde frontend 40 dosya/332 test, TypeScript kontrolü ve Vite üretim build'i geçti. Rust standart paralel koşusunda bir test `BudgetExceeded` yerine `Cancelled` gördü: duplicate testleri ortak generation sayacını etkiliyor. Aynı test tek başına geçti; yukarıdaki sıralı komutla 57/57 geçti. Bu bilinen test izolasyonu sorunu giderilmiş sayılmıyor; bu nedenle yalnızca “tüm Rust testleri paralel geçti” denmemelidir.

1 Kasım WebContent kurtarma testi tarihsel kanıttır; 20 Kasım'da gerçek kullanıcı dosyalarıyla silme veya GPU çökmesi tekrarlanmadı. Bu kontroller tüm özelliklerin her macOS sürümünde uçtan uca test edildiği anlamına gelmez.
