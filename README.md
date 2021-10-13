# twitter-pinned

CLI tool to fetch pinned tweets for specific Twitter user IDs, using the
unofficial Twitter GraphQL API.

## Usage

```sh
twitter-pinned --pretty 2955297975 746522681584066560
```

Output:

```json
[
  {
    "screen_name": "walfieee",
    "created_at": "Sun Dec 09 16:24:28 +0000 2018",
    "tweet_id": "1071802770490093569",
    "user_id": "2955297975",
    "text": "Volume 2 of LINE stickers is here!!\nLINEスタンプ第2弾ｷﾀ―――(ﾟ∀ﾟ)―――― !!\n\nhttps://t.co/mSeSEUh1Or https://t.co/o77tYzGyPl",
    "images": [
      {
        "url": "https://pbs.twimg.com/ext_tw_video_thumb/1071802113792065536/pu/img/s-I1vi0je3RMrYmk.jpg",
        "width": 742,
        "height": 720
      }
    ]
  },
  {
    "screen_name": "walfington",
    "created_at": "Mon Jan 04 03:21:55 +0000 2021",
    "tweet_id": "1345933448989585411",
    "user_id": "746522681584066560",
    "text": "gonna repurpose this videogame-related tweets account to a general purpose \"tweets that I think are too mundane for me to post on main because I have way too many followers\" account",
    "images": []
  }
]
```

