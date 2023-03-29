require 'httparty'
require 'csv'
require 'byebug'
require 'benchmark'
require 'redis'

LAMPORTS_PER_RENEC = 1_000_000_000.0
SLOTS_PER_MIN = 163
THREADS_NUM = 200

def calculate(redis, from_block, end_block)
  return if redis.get(from_block)

  voting_tx = 0
  non_voting_tx = 0

  result = nil
  for block in from_block..end_block do
    res = HTTParty.post("https://api-mainnet-beta.renec.foundation", headers: { 'Content-Type': 'application/json' }, body: {"jsonrpc": "2.0", "id": 1, "method": "getBlock", "params":[block]}.to_json)
    result = JSON.parse(res.body)["result"]
    next unless result

    transactions = result["transactions"]
    transactions.each do |tx|
      if tx["transaction"]["message"]["accountKeys"].last == "Vote111111111111111111111111111111111111111"
        voting_tx += 1
      else
        non_voting_tx += 1
      end
    end
  end
  redis.set(from_block, "#{non_voting_tx}|#{voting_tx}|#{end_block}")
rescue => e
  p "#{from_block}..#{end_block}: #{e.inspect}"
end

def calculate_with_multiple_threads(from_block)
  threads = []
  redis = Redis.new
  THREADS_NUM.times do |i|
    threads << Thread.new do
      calculate(redis, from_block + i * SLOTS_PER_MIN, from_block + (i+1) * SLOTS_PER_MIN - 1)
    end
  end
  threads.map(&:join)
end

def main
  hash_data = {}
  while true do
    from_block = 18410109
    end_block = 27580000
    non_voting_count = 0
    voting_count = 0
    redis = Redis.new
    missing_blocks = []
    while from_block < end_block do
      data = redis.get(from_block)
      unless data
        missing_blocks << from_block
      else
        hash_data[from_block] = data
        non_voting_tx, voting_tx, _ = data.split("|")
        non_voting_count += non_voting_tx.to_i
        voting_count += voting_tx.to_i
      end
      from_block += SLOTS_PER_MIN
    end
    p "Non voting: #{non_voting_count}"
    p "Voting: #{voting_count}"
    p "Total missing blocks: #{missing_blocks.count}"
    return File.write("hash_data.txt", hash_data.to_json)
    return if missing_blocks.count == 0

    threads = []
    redis = Redis.new
    50.times do |i|
      threads << Thread.new do
        calculate(redis, missing_blocks[i], missing_blocks[i] + SLOTS_PER_MIN - 1)
      end
    end
    threads.map(&:join)
  end
end

main
