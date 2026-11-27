> **Topluluk fork'u:** Bu depo [orijinal Cardinal](https://github.com/cardisoft/cardinal) üzerine geliştirilmiştir. Varsayılan dal **`public/cardinal-enhancements`**, [EminSet/cardinal](https://github.com/EminSet/cardinal) katkıları dahil tüm ek özellikleri içerir. [Ayrıntılı farklar ve entegrasyon rehberi](../../docs/FORK-INTEGRATION.tr-TR.md).

## Bu fork ile çalışma

```bash
git clone --branch public/cardinal-enhancements https://github.com/MehmetEminBozdag/cardinal.git
cd cardinal
```

Görsel tür/tarih/boyut filtreleri, en yeniler, kayıtlı aramalar, favoriler, arama kapsamı açıklamaları, genişletilmiş menüler, doğrudan Çöp'e taşıma, APFS farkındalıklı kopya analizi ve WebKit arayüz kurtarması eklenmiştir. Çekirdek sorgu eşleştirmesinin hızlandığı iddia edilmez; performans çalışması büyük sonuçların metadata sıralaması, iptal ve istek yığılmasını önleme üzerindedir.

Güncel Apple Silicon paketi: [v0.1.24-fork.2 — 27 Kasım 2026](https://github.com/MehmetEminBozdag/cardinal/releases/tag/v0.1.24-fork.2).

- **Boyut:** klasör toplamları, treemap, boş disk alanı, Çöp boyutu ve temizlik önerileri. Ayrılmış alan ve mantıksal dosya boyutu farklı olabilir; toplam boyut, APFS üzerinde kesin boşalacak alan değildir.
- **Uygulamalar:** kaldırma iş akışı ve artık yüklü olmayan uygulamalara ait olabilecek kalıntı önerileri. Çöp'e taşımadan önce önerilen yolları kontrol edin.
- **İşlemler:** Çöp'e taşıma geçmişi oturumlar arasında saklanır ve günlük boyutu sınırlıdır; geri alma özelliği değildir.
- **Arka plan CPU kontrolü:** boyut taraması yalnız Boyut sekmesi seçiliyken ve Cardinal ön plandayken çalışır. Başka sekmeye geçince hemen duraklar. Boyut seçiliyken uygulama arka plana alınırsa **15 saniye** devam edebilir; süre sonunda tam ve artımlı taramalar iptal edilir. Bekleyen değişiklikler devam etmek için korunur. Normal dosya adı indekslemesi ayrı çalışır.

EminSet'in [`a4658a1`](https://github.com/EminSet/cardinal/commit/a4658a17ffabd708dda08508aee93789f3aeaa30) commitine kadarki eklemeleri alınmıştır; boyut taramasının etkinlik kontrolü ve regresyon testleri bunun üzerine eklenmiştir. Ayrıntılar ve doğrulama sınırları [değişiklik günlüğündedir](../../CHANGELOG.md).

27 Kasım 2026 kontrolünde upstream `master` hâlâ `4c50734f` commitindedir; [PR #228](https://github.com/cardisoft/cardinal/pull/228) açıktır ve birleştirilmemiştir. `0.1.24` uygulamanın kendi içindeki sürüm numarasıdır; yeni fork paketini `v0.1.24-fork.2` etiketi ayırt eder. Birleştirme yapacak geliştirici için dal seçimi, kaynak haritası, veri/IPC sınırları ve test durumları [rehberde](../../docs/FORK-INTEGRATION.tr-TR.md) açıklanır.

---

<div align="center">
  <img src="../../cardinal/mac-icon_1024x1024.png" alt="Cardinal simgesi" width="120" height="120">
  <h1>Cardinal</h1>
  <p>Cardinal tabanlı, iş akışı ve dayanıklılık geliştirmeleri içeren macOS dosya araması.</p>
  <p>
    <a href="#cardinali-kullanma">Cardinal’i kullanma</a> ·
    <a href="#cardinali-derleme">Cardinal’i derleme</a>
  </p>
  <img src="UI.gif" alt="Cardinal arayüz önizlemesi" width="720">
</div>

---

[English](../../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## Cardinal’i kullanma

### İndirme

**Aşağıdaki Homebrew komutu ve upstream sürümleri orijinal Cardinal'i kurar; bu fork'u kurmaz.** Apple Silicon DMG için [güncel fork yayınını](https://github.com/MehmetEminBozdag/cardinal/releases/latest) kullanın; kaynak için `public/cardinal-enhancements` dalını derleyin. Paket yayımlanan topluluk fork sürümüdür, ad-hoc imzalıdır ve Apple notarization içermez. Yayın etiketi fork kaydını ayırır; uygulama içindeki native sürüm hâlâ `0.1.24` görünür.

Orijinal Cardinal'i Homebrew ile kurun:

```bash
brew install --cask cardinal-search
```

Ayrıca en güncel paketleri [GitHub Releases](https://github.com/cardisoft/cardinal/releases/) üzerinden indirebilirsiniz.

### i18n desteği

Farklı bir dil mi lazım? Durum çubuğundaki ⚙️ düğmesine tıklayarak anında değiştirin.

### Arama temelleri

Cardinal artık klasik alt dize/ön ek eşleştirmesinin üzerine Everything uyumlu bir söz dizimi katmanı ekliyor:

- `report draft` – boşluk `AND` gibi davranır; yalnızca adında her iki belirteci de içeren dosyaları görürsünüz.
- `*.pdf briefing` – adında “briefing” geçen PDF sonuçlarını filtreler.
- `*.zip size:>100MB` – 100MB’den büyük ZIP dosyalarını arar.
- `in:/Users demo !.psd` – arama kökünü `/Users` ile sınırlar; ardından adında `demo` olanları bulur ama `.psd`’yi hariç tutar.
- `tag:ProjectA;ProjectB` – Finder etiketlerini (macOS) eşleştirir; `;` `OR` gibi çalışır.
- `*.md content:"Bearer "` – `Bearer ` dizgesini içeren Markdown dosyalarını filtreler.
- `"Application Support"` – tam ifadeler için tırnak kullanın.
- `brary/Applicat` – alt yol araması için `/` yol ayırıcısı olarak kullanılır; `Library/Application Support` gibi dizinlerle eşleşir.
- `/report` · `draft/` · `/report/` – belirtecin başına/sonuna `/` ekleyerek Everything söz diziminin ötesinde, ön ek, son ek veya tam ad eşleştirmesini zorlayın.
- `~/**/.DS_Store` – globstar (`**`) ev dizininizin altındaki tüm klasörlere iner ve her yerdeki `.DS_Store` dosyalarını bulur.

Desteklenen operatör kataloğu (mantıksal gruplama, klasör kapsamı, uzantı filtreleri, regex kullanımı ve daha fazla örnek) için [`search-syntax.tr-TR.md`](search-syntax.tr-TR.md) dosyasına bakın.

### Klavye kısayolları ve önizlemeler

- `Cmd+Shift+Space` – hızlı başlatma kısayoluyla Cardinal penceresini global olarak aç/kapatın.
- `Cmd+,` – Tercihler’i açın.
- `Esc` – Cardinal penceresini gizleyin.
- `ArrowUp`/`ArrowDown` – seçimi hareket ettirin.
- `Shift+ArrowUp`/`Shift+ArrowDown` – seçimi genişletin.
- `Space` – Cardinal’den ayrılmadan seçili satırı Quick Look ile önizleyin.
- `Cmd+O` – seçili sonucu açın.
- `Cmd+R` – vurgulanan sonucu Finder’da gösterin.
- `Cmd+C` – seçili dosyaları panoya kopyalayın.
- `Cmd+Shift+C` – seçili yolları panoya kopyalayın.
- `Cmd+F` – odağı arama çubuğuna geri alın.
- `ArrowUp`/`ArrowDown` (arama çubuğunda) – arama geçmişinde gezin.

İyi aramalar!

---

## Cardinal’i derleme

### Gereksinimler

- macOS 12+
- Rust toolchain
- npm’li Node.js 18+
- Xcode command-line tools ve Tauri prerequisites (<https://tauri.app/start/prerequisites/>)

### Geliştirme modu

```bash
cd cardinal
npm ci
npm run tauri dev -- --release --features dev
```

### Üretim derlemesi

```bash
cd cardinal
npm ci
npm run tauri build
```

Rust sürümünü `rust-toolchain.toml` belirler (bu kayıtta `nightly-2025-12-11`). Paket çıktıları depo köküne göre `cardinal/src-tauri/target/release/bundle/` altındadır; Git'e dahil edilmez.
