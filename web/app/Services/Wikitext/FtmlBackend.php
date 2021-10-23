<?php
declare(strict_types=1);

namespace Wikijump\Services\Wikitext;

use Wikijump\Services\Wikitext\FFI\FtmlFfi;

/**
 * Class FtmlInterface, implements a compatible interface for working with FTML.
 * @package Wikijump\Services\Wikitext
 */
class FtmlBackend extends WikitextBackend
{
    private WikitextSettings $settings;
    private PageInfo $page_info;

    public function __construct(int $mode, ?PageInfo $page_info)
    {
        $this->settings = WikitextSettings::fromMode($mode);
        $this->page_info = $pageInfo ?? self::defaultPageInfo();
    }

    // Interface methods
    public function renderHtml(string $wikitext): HtmlOutput
    {
        return FtmlFfi::renderHtml($wikitext, $this->page_info, $this->settings);
    }

    public function renderText(string $wikitext): TextOutput
    {
        return FtmlFfi::renderText($wikitext, $this->page_info, $this->settings);
    }

    public function version(): string
    {
        return FtmlFfi::version();
    }

    private static function defaultPageInfo(): PageInfo
    {
        return new PageInfo(
            '_anonymous',
            null,
            'test',
            '_anonymous',
            null,
            [],
            'default',
        );
    }
}
