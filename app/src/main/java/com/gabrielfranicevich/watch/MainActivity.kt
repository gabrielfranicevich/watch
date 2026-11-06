package com.gabrielfranicevich.watch

import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.media3.common.MediaItem
import androidx.media3.common.MimeTypes
import androidx.media3.datasource.DefaultDataSource
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.exoplayer.source.MergingMediaSource
import androidx.media3.ui.PlayerView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

data class AnimeSource(
  val provider: String,
  val providerName: String,
  val id: String,
  val title: String,
  val url: String
)

data class AnimeChoice(
  val label: String,
  val sources: List<AnimeSource>
)

data class PlaybackInfo(
  val streamUrl: String,
  val referer: String?,
  val audioUrl: String?,
  val subtitleUrl: String?
)

data class EpisodeChoice(
  val provider: String,
  val number: String,
  val title: String?,
  val url: String
)

class MainActivity : ComponentActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    setContent {
      MaterialTheme {
        Surface(modifier = Modifier.fillMaxSize()) {
          WatchApp()
        }
      }
    }
  }
}

@androidx.compose.runtime.Composable
private fun WatchApp() {
  val context = LocalContext.current
  val scope = rememberCoroutineScope()
  val native = remember {
    NativeWatch(context.filesDir.resolve("history/ani-hsts").absolutePath)
  }
  DisposableEffect(native) {
    onDispose { native.close() }
  }

  var query by remember { mutableStateOf("") }
  var language by remember { mutableStateOf("") }
  var results by remember { mutableStateOf(emptyList<AnimeChoice>()) }
  var episodes by remember { mutableStateOf(emptyList<EpisodeChoice>()) }
  var selectedAnime by remember { mutableStateOf<String?>(null) }
  var playback by remember { mutableStateOf<PlaybackInfo?>(null) }
  var error by remember { mutableStateOf<String?>(null) }
  var busy by remember { mutableStateOf(false) }

  fun search() {
    if (query.isBlank() || busy) return
    busy = true
    error = null
    episodes = emptyList()
    selectedAnime = null
    playback = null
    scope.launch {
      try {
        results = withContext(Dispatchers.IO) {
          parseSearch(native.search(query.trim(), language.ifEmpty { null }))
        }
        if (results.isEmpty()) error = "No results found"
      } catch (failure: Exception) {
        error = failure.message ?: "Search failed"
      } finally {
        busy = false
      }
    }
  }

  fun selectAnime(choice: AnimeChoice) {
    if (busy) return
    busy = true
    error = null
    selectedAnime = choice.label
    playback = null
    episodes = emptyList()
    scope.launch {
      try {
        episodes = withContext(Dispatchers.IO) {
          val loaded = mutableListOf<EpisodeChoice>()
          choice.sources.forEach { source ->
            try {
              loaded += parseEpisodes(native.episodes(
                source.provider,
                source.id,
                source.title,
                source.url
              ), source.provider)
            } catch (_: Exception) {
              Unit
            }
          }
          loaded.distinctBy { "${it.provider}:${it.number}:${it.url}" }
        }
        if (episodes.isEmpty()) error = "No episodes could be loaded from the available sources"
      } catch (failure: Exception) {
        error = failure.message ?: "Episode loading failed"
      } finally {
        busy = false
      }
    }
  }

  fun selectEpisode(episode: EpisodeChoice) {
    if (busy) return
    busy = true
    error = null
    playback = null
    scope.launch {
      try {
        val json = withContext(Dispatchers.IO) {
          native.resolve(
            episode.provider,
            episode.number,
            episode.title,
            episode.url
          ).also(::throwIfBridgeError)
        }
        playback = parsePlayback(json)
      } catch (failure: Exception) {
        error = failure.message ?: "Could not resolve this episode"
      } finally {
        busy = false
      }
    }
  }

  Column(
    modifier = Modifier
      .fillMaxSize()
      .padding(16.dp),
    verticalArrangement = Arrangement.spacedBy(12.dp)
  ) {
    Text("Watch", style = MaterialTheme.typography.headlineLarge)
    OutlinedTextField(
      value = query,
      onValueChange = { query = it },
      modifier = Modifier.fillMaxWidth(),
      label = { Text("Anime, movie or series") },
      singleLine = true
    )
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
      FilterChip(selected = language.isEmpty(), onClick = { language = "" }, label = { Text("All") })
      FilterChip(selected = language == "en", onClick = { language = "en" }, label = { Text("English") })
      FilterChip(selected = language == "es", onClick = { language = "es" }, label = { Text("Spanish") })
      Button(onClick = ::search, enabled = query.isNotBlank() && !busy) {
        Text("Search")
      }
    }
    if (busy) CircularProgressIndicator()
    error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    selectedAnime?.let {
      Text(it, style = MaterialTheme.typography.titleMedium)
    }
    LazyColumn(
      modifier = Modifier.weight(1f),
      verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
      if (selectedAnime == null) {
        items(results) { choice ->
          Card(
            modifier = Modifier
              .fillMaxWidth()
              .clickable(enabled = !busy) { selectAnime(choice) }
          ) {
            Column(Modifier.padding(12.dp)) {
              Text(choice.label, style = MaterialTheme.typography.titleMedium)
              Text(choice.sources.joinToString { it.providerName }, style = MaterialTheme.typography.bodySmall)
            }
          }
        }
      } else {
        items(episodes) { episode ->
          Card(
            modifier = Modifier
              .fillMaxWidth()
              .clickable(enabled = !busy) { selectEpisode(episode) }
          ) {
            Column(Modifier.padding(12.dp)) {
              Text("Episode ${episode.number}", style = MaterialTheme.typography.titleMedium)
              episode.title?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
              Text(episode.provider, style = MaterialTheme.typography.labelSmall)
            }
          }
        }
      }
    }
    playback?.let { NativePlayer(it) }
  }
}

private fun parseSearch(json: String): List<AnimeChoice> {
  val root = JSONObject(json)
  throwIfBridgeError(root)
  val results = root.getJSONArray("results")
  return List(results.length()) { index ->
    val item = results.getJSONObject(index)
    val sources = item.getJSONArray("sources")
    AnimeChoice(
      label = item.getString("label"),
      sources = List(sources.length()) { sourceIndex ->
        val source = sources.getJSONObject(sourceIndex)
        AnimeSource(
          provider = source.getString("provider"),
          providerName = source.getString("provider_name"),
          id = source.getString("id"),
          title = source.getString("title"),
          url = source.getString("url")
        )
      }
    )
  }
}

private fun parseEpisodes(json: String, provider: String): List<EpisodeChoice> {
  val root = JSONObject(json)
  throwIfBridgeError(root)
  val episodes = root.getJSONArray("episodes")
  return List(episodes.length()) { index ->
    val item = episodes.getJSONObject(index)
    EpisodeChoice(
      provider = provider,
      number = item.getString("number"),
      title = item.optString("title").takeIf { it.isNotBlank() && it != "null" },
      url = item.getString("url")
    )
  }
}

private fun throwIfBridgeError(json: String) {
  throwIfBridgeError(JSONObject(json))
}

private fun throwIfBridgeError(json: JSONObject) {
  if (json.has("error")) throw IllegalStateException(json.getString("error"))
}

private fun parsePlayback(json: String): PlaybackInfo {
  val root = JSONObject(json)
  throwIfBridgeError(root)
  val stream = root.getJSONObject("stream")
  val subtitle = root.optJSONObject("subtitle")
  return PlaybackInfo(
    streamUrl = stream.getString("url"),
    referer = stream.optString("referer").takeIf { it.isNotBlank() && it != "null" },
    audioUrl = stream.optString("audio_url").takeIf { it.isNotBlank() && it != "null" },
    subtitleUrl = subtitle?.optString("url")?.takeIf { it.isNotBlank() && it != "null" }
  )
}

@androidx.compose.runtime.Composable
private fun NativePlayer(info: PlaybackInfo) {
  val context = LocalContext.current
  val player = remember(info) {
    val headers = info.referer?.let { mapOf("Referer" to it) }.orEmpty()
    val httpFactory = DefaultHttpDataSource.Factory().setDefaultRequestProperties(headers)
    val dataSourceFactory = DefaultDataSource.Factory(context, httpFactory)
    val mediaSourceFactory = DefaultMediaSourceFactory(dataSourceFactory)
    val itemBuilder = MediaItem.Builder().setUri(info.streamUrl)
    info.subtitleUrl?.let { subtitleUrl ->
      val mimeType = when {
        subtitleUrl.substringBefore('?').endsWith(".vtt", ignoreCase = true) -> MimeTypes.TEXT_VTT
        subtitleUrl.substringBefore('?').endsWith(".srt", ignoreCase = true) -> MimeTypes.APPLICATION_SUBRIP
        else -> null
      }
      if (mimeType != null) {
        itemBuilder.setSubtitleConfigurations(
          listOf(
            MediaItem.SubtitleConfiguration.Builder(Uri.parse(subtitleUrl))
              .setMimeType(mimeType)
              .build()
          )
        )
      }
    }
    val exoPlayer = ExoPlayer.Builder(context)
      .setMediaSourceFactory(mediaSourceFactory)
      .build()
    val video = mediaSourceFactory.createMediaSource(itemBuilder.build())
    val source = info.audioUrl?.let { audioUrl ->
      MergingMediaSource(
        video,
        mediaSourceFactory.createMediaSource(MediaItem.fromUri(audioUrl))
      )
    } ?: video
    exoPlayer.setMediaSource(source)
    exoPlayer.prepare()
    exoPlayer.playWhenReady = true
    exoPlayer
  }

  androidx.compose.runtime.DisposableEffect(player) {
    onDispose { player.release() }
  }

  AndroidView(
    modifier = Modifier
      .fillMaxWidth()
      .height(220.dp),
    factory = { viewContext ->
      PlayerView(viewContext).apply {
        this.player = player
        useController = true
      }
    },
    update = { it.player = player }
  )
}
