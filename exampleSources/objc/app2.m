// Objective-C — NSDictionary + blocks for transformation.

#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        NSDictionary *counts = @{
            @"rust":   @42,
            @"scala":  @27,
            @"python": @35,
            @"go":     @18,
        };

        NSArray *langs = [counts keysSortedByValueUsingComparator:^NSComparisonResult(NSNumber *a, NSNumber *b) {
            return [b compare:a];
        }];

        NSUInteger total = 0;
        for (NSNumber *n in [counts allValues]) {
            total += [n unsignedIntegerValue];
        }

        NSLog(@"total = %lu", (unsigned long)total);
        for (NSString *lang in langs) {
            NSLog(@"%@ -> %@", lang, counts[lang]);
        }
    }
    return 0;
}